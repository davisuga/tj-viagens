use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{append_audit_tx, AuditInput};
use crate::auth::AuthUser;
use crate::domain::types::QuotationStatus;
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::App;

use super::quotations::{load_quotation, require_active_supplier};
use super::views::effective_status;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposalBody {
    total_price_cents: i64,
    flight_info: String,
    notes: Option<String>,
}

async fn submit(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ProposalBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let supplier_id = require_active_supplier(&state.pool, &claims).await?;
    if body.total_price_cents <= 0 {
        return Err(ApiError::Unprocessable("PRECO_INVALIDO"));
    }
    if body.flight_info.trim().len() < 2 {
        return Err(ApiError::Unprocessable("VOO_INVALIDO"));
    }
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    match effective_status(&q.status, q.closes_at, now) {
        QuotationStatus::Open => {}
        QuotationStatus::Draft
        | QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => return Err(ApiError::Unprocessable("COTACAO_FECHADA")),
    }
    // Replace-while-open semantics: one row per supplier, first submitted_at preserved.
    // Bid + audit entry commit ATOMICALLY — a bid can never exist without its trail row.
    let mut tx = state.pool.begin().await?;
    let (proposal_id, submitted_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO proposals (id, quotation_id, supplier_id, total_price_cents, flight_info, notes) \
         VALUES ($1,$2,$3,$4,$5,$6) \
         ON CONFLICT (quotation_id, supplier_id) DO UPDATE \
         SET total_price_cents = EXCLUDED.total_price_cents, \
             flight_info = EXCLUDED.flight_info, \
             notes = EXCLUDED.notes, \
             updated_at = now() \
         RETURNING id, submitted_at",
    )
    .bind(Uuid::new_v4()).bind(id).bind(supplier_id)
    .bind(body.total_price_cents).bind(&body.flight_info).bind(&body.notes)
    .fetch_one(&mut *tx)
    .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "PROPOSAL_SUBMITTED",
        entity: "Proposal",
        entity_id: proposal_id.to_string(),
        quotation_id: Some(id),
        payload: json!({ "totalPriceCents": body.total_price_cents, "flightInfo": body.flight_info }),
    })
    .await?;
    tx.commit().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    // Sealed bids: the live event carries only the COUNT.
    publish(&state, id, "proposal", json!({ "count": count }));
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": proposal_id,
            "totalPriceCents": body.total_price_cents,
            "submittedAt": submitted_at.to_rfc3339()
        })),
    ))
}

pub fn router() -> Router<App> {
    Router::new().route("/quotations/{id}/proposals", post(submit))
}
