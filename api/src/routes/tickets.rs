use std::collections::HashMap;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{append_audit_tx, AuditInput};
use crate::auth::{AuthUser, Staff};
use crate::domain::divergence::{compute_divergences, TicketFields};
use crate::domain::types::QuotationStatus;
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::uploads::save_upload;
use crate::App;

use super::quotations::{fetch_proposals, fetch_quotation};
use super::suppliers::require_supplier;

async fn upload_ticket(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let supplier_id = require_supplier(&claims)?;
    let now = Utc::now();
    let Some(q) = fetch_quotation(&state.pool, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    if QuotationStatus::parse(&q.status) != Some(QuotationStatus::Awarded) {
        return Err(ApiError::Unprocessable("NAO_AGUARDA_BILHETE"));
    }
    let proposals = fetch_proposals(&state.pool, id).await?;
    let Some(winner) = proposals.iter().find(|p| Some(p.id) == q.awarded_proposal_id) else {
        return Err(ApiError::Internal("awarded quotation without winner proposal".into()));
    };
    if winner.supplier_id != supplier_id {
        return Err(ApiError::Forbidden("ACESSO_NEGADO"));
    }

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut saved: Option<(String, String)> = None;
    while let Some(field) =
        multipart.next_field().await.map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let original = field.file_name().unwrap_or("eticket.pdf").to_string();
            let bytes = field.bytes().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            saved = Some(save_upload(&state.config.upload_dir, &original, &bytes).await?);
        } else {
            let value = field.text().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            fields.insert(name, value);
        }
    }
    let passenger_name =
        fields.get("passengerName").cloned().ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let flight_info =
        fields.get("flightInfo").cloned().ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let departure_at: DateTime<Utc> = fields
        .get("departureAt")
        .and_then(|v| v.parse().ok())
        .ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let price_cents: i64 = fields
        .get("priceCents")
        .and_then(|v| v.parse().ok())
        .ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    if passenger_name.trim().len() < 3 || passenger_name.chars().count() > 200 {
        return Err(ApiError::Unprocessable("BILHETE_INVALIDO"));
    }
    if flight_info.trim().len() < 2 || flight_info.chars().count() > 200 {
        return Err(ApiError::Unprocessable("BILHETE_INVALIDO"));
    }
    if price_cents <= 0 || price_cents > 1_000_000_000 {
        return Err(ApiError::Unprocessable("BILHETE_INVALIDO"));
    }
    // Written before the tx below: on a rolled-back tx (e.g. a lost double-upload
    // race) this file is orphaned on disk. Acceptable for the prototype.
    let Some((file_name, file_path)) = saved else {
        return Err(ApiError::Unprocessable("BILHETE_INVALIDO"));
    };

    let ticket_fields = TicketFields { passenger_name: &passenger_name, departure_at, price_cents };
    let divergences = compute_divergences(
        &q.passenger_name,
        q.departure_at,
        winner.total_price_cents,
        &ticket_fields,
    );
    // R8 as KPI, not hard block: accept late uploads but flag them.
    let late = matches!(q.ticket_deadline_at, Some(d) if now > d);
    // Ticket row + status flip + audit commit ATOMICALLY; the guarded UPDATE makes a
    // double-upload lose with 422 instead of hitting the tickets unique constraint.
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE quotations SET status = 'TICKETED' WHERE id = $1 AND status = 'AWARDED'",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Unprocessable("NAO_AGUARDA_BILHETE"));
    }
    let ticket_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tickets (id, quotation_id, file_name, file_path, passenger_name, \
         flight_info, departure_at, price_cents, divergences, late) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(ticket_id).bind(id).bind(&file_name).bind(&file_path).bind(&passenger_name)
    .bind(&flight_info).bind(departure_at).bind(price_cents)
    .bind(json!(divergences)).bind(late)
    .execute(&mut *tx)
    .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "TICKET_UPLOADED",
        entity: "Ticket",
        entity_id: ticket_id.to_string(),
        quotation_id: Some(id),
        payload: json!({ "late": late, "divergences": divergences, "priceCents": price_cents }),
    })
    .await?;
    tx.commit().await?;
    publish(&state, id, "status", json!({ "status": "TICKETED" }));
    Ok((StatusCode::CREATED, Json(json!({ "id": ticket_id, "late": late, "divergences": divergences }))))
}

async fn confirm_ticket(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let ticket: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM tickets WHERE quotation_id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((ticket_id,)) = ticket else { return Err(ApiError::NotFound("BILHETE_NAO_ENVIADO")) };
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE quotations SET status = 'COMPLETED' WHERE id = $1 AND status = 'TICKETED'",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Unprocessable("STATUS_INVALIDO"));
    }
    sqlx::query("UPDATE tickets SET confirmed_at = now(), confirmed_by = $1 WHERE id = $2")
        .bind(claims.sub)
        .bind(ticket_id)
        .execute(&mut *tx)
        .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "TICKET_CONFIRMED",
        entity: "Ticket",
        entity_id: ticket_id.to_string(),
        quotation_id: Some(id),
        payload: json!({}),
    })
    .await?;
    tx.commit().await?;
    publish(&state, id, "status", json!({ "status": "COMPLETED" }));
    Ok(Json(json!({ "status": "COMPLETED" })))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/quotations/{id}/ticket", post(upload_ticket))
        .route("/quotations/{id}/ticket/confirm", post(confirm_ticket))
}
