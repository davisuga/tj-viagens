use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{append_audit, append_audit_tx, AuditInput};
use crate::auth::{AuthUser, Claims, Staff};
use crate::domain::timefmt::fmt_boa_vista;
use crate::domain::types::{QuotationStatus, Role, SupplierStatus};
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::App;

use super::suppliers::require_supplier;
use super::views::{effective_status, staff_view, supplier_view, ProposalRow, QuotationRow};

pub const QUOTATION_COLUMNS: &str =
    "id, code, status, passenger_name, passenger_cpf, passenger_sex, passenger_birth, \
     origin, destination, departure_at, return_at, reference_flight, reference_price_cents, \
     opens_at, closes_at, awarded_proposal_id, awarded_at, award_justification, \
     ticket_deadline_at, created_by, created_at";

pub async fn fetch_quotation(pool: &PgPool, id: Uuid) -> ApiResult<Option<QuotationRow>> {
    Ok(sqlx::query_as::<_, QuotationRow>(&format!(
        "SELECT {QUOTATION_COLUMNS} FROM quotations WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

/// Always ordered (price asc, submitted_at asc) — this IS the ranking order (R6).
pub async fn fetch_proposals(pool: &PgPool, quotation_id: Uuid) -> ApiResult<Vec<ProposalRow>> {
    Ok(sqlx::query_as::<_, ProposalRow>(
        "SELECT id, quotation_id, supplier_id, total_price_cents, flight_info, notes, submitted_at \
         FROM proposals WHERE quotation_id = $1 \
         ORDER BY total_price_cents ASC, submitted_at ASC",
    )
    .bind(quotation_id)
    .fetch_all(pool)
    .await?)
}

/// Lazily persists OPEN -> CLOSED once the server clock passes closes_at (R4).
pub async fn load_quotation(
    state: &App,
    id: Uuid,
    now: DateTime<Utc>,
) -> ApiResult<Option<QuotationRow>> {
    let Some(q) = fetch_quotation(&state.pool, id).await? else { return Ok(None) };
    if q.status == "OPEN"
        && effective_status(&q.status, q.closes_at, now) == QuotationStatus::Closed
    {
        let updated =
            sqlx::query("UPDATE quotations SET status = 'CLOSED' WHERE id = $1 AND status = 'OPEN'")
                .bind(id)
                .execute(&state.pool)
                .await?;
        if updated.rows_affected() == 1 {
            append_audit(&state.pool, AuditInput {
                actor_id: None,
                actor_role: None,
                event_type: "QUOTATION_CLOSED",
                entity: "Quotation",
                entity_id: id.to_string(),
                quotation_id: Some(id),
                payload: json!({ "closesAt": q.closes_at.map(|d| d.to_rfc3339()) }),
            })
            .await?;
            publish(state, id, "status", json!({ "status": "CLOSED" }));
        }
        return fetch_quotation(&state.pool, id).await;
    }
    Ok(Some(q))
}

/// Atomic sequential codes: COT-2026-0001, OS-2026-0001.
pub async fn next_code<'e, E: sqlx::PgExecutor<'e>>(executor: E, prefix: &str) -> ApiResult<String> {
    let key = format!("{prefix}-{}", Utc::now().format("%Y"));
    let value: i64 = sqlx::query_scalar(
        "INSERT INTO counters (id, value) VALUES ($1, 1) \
         ON CONFLICT (id) DO UPDATE SET value = counters.value + 1 RETURNING value",
    )
    .bind(&key)
    .fetch_one(executor)
    .await?;
    Ok(format!("{key}-{value:04}"))
}

pub async fn require_active_supplier(pool: &PgPool, claims: &Claims) -> ApiResult<Uuid> {
    let supplier_id = require_supplier(claims)?;
    let status: Option<(String,)> = sqlx::query_as("SELECT status FROM suppliers WHERE id = $1")
        .bind(supplier_id)
        .fetch_optional(pool)
        .await?;
    match status.and_then(|(s,)| SupplierStatus::parse(&s)) {
        Some(SupplierStatus::Active) => Ok(supplier_id),
        Some(SupplierStatus::Pending)
        | Some(SupplierStatus::Rejected)
        | Some(SupplierStatus::Suspended)
        | None => Err(ApiError::Forbidden("FORNECEDOR_NAO_ATIVO")),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBody {
    passenger_name: String,
    passenger_cpf: String,
    passenger_sex: String,
    passenger_birth: NaiveDate,
    origin: String,
    destination: String,
    departure_at: DateTime<Utc>,
    return_at: Option<DateTime<Utc>>,
    reference_flight: String,
    reference_price_cents: i64,
}

async fn create(
    State(state): State<App>,
    Staff(claims): Staff,
    Json(body): Json<CreateBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let cpf: String = body.passenger_cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if cpf.len() != 11 {
        return Err(ApiError::Unprocessable("CPF_INVALIDO"));
    }
    if !matches!(body.passenger_sex.as_str(), "F" | "M" | "O") {
        return Err(ApiError::Unprocessable("SEXO_INVALIDO"));
    }
    if body.reference_price_cents <= 0 {
        return Err(ApiError::Unprocessable("PRECO_INVALIDO"));
    }
    let id = Uuid::new_v4();
    let code = next_code(&state.pool, "COT").await?;
    sqlx::query(
        "INSERT INTO quotations \
         (id, code, passenger_name, passenger_cpf, passenger_sex, passenger_birth, \
          origin, destination, departure_at, return_at, reference_flight, \
          reference_price_cents, created_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(id).bind(&code).bind(&body.passenger_name).bind(&cpf).bind(&body.passenger_sex)
    .bind(body.passenger_birth).bind(&body.origin).bind(&body.destination)
    .bind(body.departure_at).bind(body.return_at).bind(&body.reference_flight)
    .bind(body.reference_price_cents).bind(claims.sub)
    .execute(&state.pool)
    .await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "QUOTATION_CREATED",
        entity: "Quotation",
        entity_id: id.to_string(),
        quotation_id: Some(id),
        payload: json!({ "code": code }),
    })
    .await?;
    let q = fetch_quotation(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("quotation vanished after insert".into()))?;
    Ok((StatusCode::CREATED, Json(staff_view(&q, &[], Utc::now()))))
}

async fn open(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let Some(q) = fetch_quotation(&state.pool, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let now = Utc::now();
    let closes_at = now + Duration::minutes(state.config.proposal_window_minutes);
    // Status flip + notifications + audit commit ATOMICALLY; the guarded UPDATE
    // makes a double-submit lose with 422 instead of double-notifying.
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE quotations SET status = 'OPEN', opens_at = $1, closes_at = $2 \
         WHERE id = $3 AND status = 'DRAFT'",
    )
    .bind(now).bind(closes_at).bind(id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Unprocessable("NAO_ESTA_EM_RASCUNHO"));
    }
    // R3: simultaneous notification of every ACTIVE supplier. Copy is Boa Vista
    // local (UX rule: a supplier misreading UTC is the worst possible failure).
    // Never contains the reference price.
    let active: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, contact_email FROM suppliers WHERE status = 'ACTIVE'")
            .fetch_all(&mut *tx)
            .await?;
    let message = format!(
        "Nova cotação {}: {} → {}, embarque {}. Propostas até {} (horário de Boa Vista).",
        q.code,
        q.origin,
        q.destination,
        fmt_boa_vista(q.departure_at),
        fmt_boa_vista(closes_at)
    );
    for (supplier_id, _email) in &active {
        sqlx::query(
            "INSERT INTO notifications (id, supplier_id, quotation_id, kind, message) \
             VALUES ($1,$2,$3,'COTACAO_ABERTA',$4)",
        )
        .bind(Uuid::new_v4()).bind(supplier_id).bind(id).bind(&message)
        .execute(&mut *tx)
        .await?;
    }
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "QUOTATION_OPENED",
        entity: "Quotation",
        entity_id: id.to_string(),
        quotation_id: Some(id),
        payload: json!({
            "code": q.code,
            "closesAt": closes_at.to_rfc3339(),
            "notified": active.len()
        }),
    })
    .await?;
    tx.commit().await?;
    // Side effects only after the durable commit.
    for (_supplier_id, email) in &active {
        // Console mail adapter — swap for institutional SMTP without touching callers.
        println!("[mail] to={email} subject=\"Cotação {} aberta\"", q.code);
    }
    publish(&state, id, "status", json!({ "status": "OPEN", "closesAt": closes_at.to_rfc3339() }));
    let q = fetch_quotation(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("quotation vanished after open".into()))?;
    Ok(Json(staff_view(&q, &[], now)))
}

async fn list(State(state): State<App>, AuthUser(claims): AuthUser) -> ApiResult<Json<Value>> {
    let now = Utc::now();
    match claims.role {
        Role::Fornecedor => {
            let supplier_id = require_active_supplier(&state.pool, &claims).await?;
            let rows = sqlx::query_as::<_, QuotationRow>(&format!(
                "SELECT {QUOTATION_COLUMNS} FROM quotations \
                 WHERE status <> 'DRAFT' AND (status = 'OPEN' OR id IN \
                   (SELECT quotation_id FROM proposals WHERE supplier_id = $1)) \
                 ORDER BY created_at DESC"
            ))
            .bind(supplier_id)
            .fetch_all(&state.pool)
            .await?;
            let mut result = Vec::new();
            for q in &rows {
                let proposals = fetch_proposals(&state.pool, q.id).await?;
                result.push(supplier_view(q, &proposals, supplier_id, now));
            }
            Ok(Json(json!(result)))
        }
        Role::Admin | Role::Servidor => {
            let rows = sqlx::query_as::<_, QuotationRow>(&format!(
                "SELECT {QUOTATION_COLUMNS} FROM quotations ORDER BY created_at DESC"
            ))
            .fetch_all(&state.pool)
            .await?;
            let mut result = Vec::new();
            for q in &rows {
                let proposals = fetch_proposals(&state.pool, q.id).await?;
                result.push(staff_view(q, &proposals, now));
            }
            Ok(Json(json!(result)))
        }
    }
}

async fn detail(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let proposals = fetch_proposals(&state.pool, id).await?;
    match claims.role {
        Role::Fornecedor => {
            let supplier_id = require_active_supplier(&state.pool, &claims).await?;
            Ok(Json(supplier_view(&q, &proposals, supplier_id, now)))
        }
        Role::Admin | Role::Servidor => Ok(Json(staff_view(&q, &proposals, now))),
    }
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/quotations", post(create).get(list))
        .route("/quotations/{id}", get(detail))
        .route("/quotations/{id}/open", post(open))
}
