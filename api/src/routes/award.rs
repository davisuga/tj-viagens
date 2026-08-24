use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{append_audit_tx, AuditInput};
use crate::auth::Staff;
use crate::domain::timefmt::fmt_boa_vista;
use crate::domain::types::QuotationStatus;
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::App;

use super::quotations::{fetch_proposals, load_quotation, next_code};
use super::views::{effective_status, staff_view};

async fn ranking(
    State(state): State<App>,
    Staff(_claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    match effective_status(&q.status, q.closes_at, now) {
        QuotationStatus::Draft | QuotationStatus::Open => {
            return Err(ApiError::Unprocessable("COTACAO_AINDA_ABERTA"))
        }
        QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => {}
    }
    let proposals = fetch_proposals(&state.pool, id).await?;
    let supplier_ids: Vec<Uuid> = proposals.iter().map(|p| p.supplier_id).collect();
    let suppliers: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, legal_name, cnpj FROM suppliers WHERE id = ANY($1)")
            .bind(&supplier_ids)
            .fetch_all(&state.pool)
            .await?;
    let by_id: HashMap<Uuid, (String, String)> =
        suppliers.into_iter().map(|(id, name, cnpj)| (id, (name, cnpj))).collect();
    let ranking: Vec<Value> = proposals
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (legal_name, cnpj) = by_id.get(&p.supplier_id).cloned().unwrap_or_default();
            json!({
                "position": i + 1,
                "proposalId": p.id,
                "supplier": { "id": p.supplier_id, "legalName": legal_name, "cnpj": cnpj },
                "totalPriceCents": p.total_price_cents,
                "flightInfo": p.flight_info,
                "notes": p.notes,
                "submittedAt": p.submitted_at.to_rfc3339(),
                "deltaFromReferenceCents": p.total_price_cents - q.reference_price_cents,
            })
        })
        .collect();
    Ok(Json(json!({
        "quotation": staff_view(&q, &proposals, now),
        "referencePriceCents": q.reference_price_cents,
        "ranking": ranking
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AwardBody {
    proposal_id: Uuid,
    justification: String,
}

async fn award(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
    Json(body): Json<AwardBody>,
) -> ApiResult<Json<Value>> {
    if body.justification.trim().len() < 5 {
        return Err(ApiError::Unprocessable("JUSTIFICATIVA_CURTA"));
    }
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    match effective_status(&q.status, q.closes_at, now) {
        QuotationStatus::Closed => {}
        QuotationStatus::Draft
        | QuotationStatus::Open
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => {
            return Err(ApiError::Unprocessable("NAO_ESTA_FECHADA"))
        }
    }
    let proposals = fetch_proposals(&state.pool, id).await?;
    let Some(winner) = proposals.iter().find(|p| p.id == body.proposal_id) else {
        return Err(ApiError::Unprocessable("PROPOSTA_INVALIDA"));
    };
    let deadline = now + Duration::minutes(state.config.ticket_window_minutes);
    // Award + OS number + both audit entries + winner notification commit ATOMICALLY.
    // The guarded UPDATE makes a double-submit lose with 422 (single-shot), and a
    // rolled-back award never burns an OS number (next_code runs inside the tx).
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE quotations SET status = 'AWARDED', awarded_proposal_id = $1, awarded_at = $2, \
         award_justification = $3, ticket_deadline_at = $4 WHERE id = $5 AND status = 'CLOSED'",
    )
    .bind(winner.id).bind(now).bind(&body.justification).bind(deadline).bind(id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Unprocessable("NAO_ESTA_FECHADA"));
    }
    let os_number = next_code(&mut *tx, "OS").await?;
    sqlx::query("INSERT INTO service_orders (id, quotation_id, number) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4()).bind(id).bind(&os_number)
        .execute(&mut *tx)
        .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "QUOTATION_AWARDED",
        entity: "Quotation",
        entity_id: id.to_string(),
        quotation_id: Some(id),
        payload: json!({
            "proposalId": winner.id.to_string(),
            "supplierId": winner.supplier_id.to_string(),
            "totalPriceCents": winner.total_price_cents,
            "justification": body.justification
        }),
    })
    .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "SERVICE_ORDER_ISSUED",
        entity: "ServiceOrder",
        entity_id: os_number.clone(),
        quotation_id: Some(id),
        payload: json!({ "number": os_number }),
    })
    .await?;
    let (winner_email,): (String,) =
        sqlx::query_as("SELECT contact_email FROM suppliers WHERE id = $1")
            .bind(winner.supplier_id)
            .fetch_one(&mut *tx)
            .await?;
    let message = format!(
        "Sua proposta venceu a cotação {}. Envie o e-ticket até {} (horário de Boa Vista).",
        q.code,
        fmt_boa_vista(deadline)
    );
    sqlx::query(
        "INSERT INTO notifications (id, supplier_id, quotation_id, kind, message) \
         VALUES ($1,$2,$3,'VENCEDORA',$4)",
    )
    .bind(Uuid::new_v4()).bind(winner.supplier_id).bind(id).bind(&message)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    // Side effects only after the durable commit.
    println!("[mail] to={winner_email} subject=\"Vencedora da cotação {}\"", q.code);
    publish(&state, id, "status", json!({ "status": "AWARDED", "ticketDeadlineAt": deadline.to_rfc3339() }));
    Ok(Json(json!({
        "serviceOrder": { "number": os_number },
        "ticketDeadlineAt": deadline.to_rfc3339()
    })))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/quotations/{id}/ranking", get(ranking))
        .route("/quotations/{id}/award", post(award))
}
