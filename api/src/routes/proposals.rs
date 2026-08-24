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
    if body.total_price_cents > 1_000_000_000 {
        return Err(ApiError::Unprocessable("PRECO_INVALIDO"));
    }
    if body.flight_info.chars().count() > 200 {
        return Err(ApiError::Unprocessable("VOO_INVALIDO"));
    }
    if body.notes.as_deref().is_some_and(|n| n.chars().count() > 2000) {
        return Err(ApiError::Unprocessable("OBSERVACOES_LONGAS"));
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
    // The INSERT..SELECT WHERE EXISTS re-checks the window IN the database, closing the
    // TOCTOU between the Rust-side check above and the commit (defense-in-depth for isonomia).
    let mut tx = state.pool.begin().await?;
    let row: Option<(Uuid, DateTime<Utc>, bool)> = sqlx::query_as(
        "INSERT INTO proposals (id, quotation_id, supplier_id, total_price_cents, flight_info, notes) \
         SELECT $1, $2, $3, $4, $5, $6 \
         WHERE EXISTS (SELECT 1 FROM quotations WHERE id = $2 AND status = 'OPEN' AND closes_at > now()) \
         ON CONFLICT (quotation_id, supplier_id) DO UPDATE \
         SET total_price_cents = EXCLUDED.total_price_cents, \
             flight_info = EXCLUDED.flight_info, \
             notes = EXCLUDED.notes, \
             updated_at = now() \
         RETURNING id, submitted_at, (xmax = 0) AS inserted",
    )
    .bind(Uuid::new_v4()).bind(id).bind(supplier_id)
    .bind(body.total_price_cents).bind(&body.flight_info).bind(&body.notes)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((proposal_id, submitted_at, inserted)) = row else {
        return Err(ApiError::Unprocessable("COTACAO_FECHADA"));
    };
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        // Transparency: a revision is audited as its own event type; the stable
        // proposal id (entity_id) links the revision history together.
        event_type: if inserted { "PROPOSAL_SUBMITTED" } else { "PROPOSAL_REPLACED" },
        entity: "Proposal",
        entity_id: proposal_id.to_string(),
        quotation_id: Some(id),
        payload: json!({
            "totalPriceCents": body.total_price_cents,
            "flightInfo": body.flight_info,
            "notes": body.notes,
        }),
    })
    .await?;
    // Count inside the tx (serialized by the audit advisory lock) so successive
    // published counts follow commit order; the UI refetches on the event anyway.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
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
