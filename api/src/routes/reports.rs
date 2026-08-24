use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{list_events, verify_chain};
use crate::auth::{AuthUser, Staff};
use crate::domain::brl::format_brl;
use crate::domain::cpf::mask_cpf;
use crate::domain::economy::compute_economy;
use crate::domain::timefmt::fmt_boa_vista;
use crate::domain::types::Role;
use crate::error::{ApiError, ApiResult};
use crate::html::{OsTemplate, ReportEvent, ReportProposal, ReportTemplate};
use crate::App;

use super::quotations::fetch_proposals;

async fn audit_verify(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    Ok(Json(verify_chain(&state.pool).await?))
}

async fn audit_events(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    let rows = list_events(&state.pool, None).await?;
    let events: Vec<Value> = rows
        .iter()
        .map(|e| {
            json!({
                "seq": e.seq, "at": e.at, "type": e.event_type, "entity": e.entity,
                "entityId": e.entity_id, "actorId": e.actor_id, "payload": e.payload
            })
        })
        .collect();
    Ok(Json(json!(events)))
}

#[allow(clippy::type_complexity)]
struct Dossier {
    q: super::views::QuotationRow,
    proposals: Vec<super::views::ProposalRow>,
    supplier_names: std::collections::HashMap<Uuid, (String, String)>,
    notified: i64,
    os: Option<(String, DateTime<Utc>)>,
    ticket: Option<(String, bool, Value, DateTime<Utc>, Option<DateTime<Utc>>)>,
}

#[allow(clippy::type_complexity)]
async fn load_dossier(state: &App, id: Uuid) -> ApiResult<Option<Dossier>> {
    // R4: lazy-close so a lapsed-OPEN quotation reports CLOSED in the dossier/report
    // pages too, not just the live quotation endpoints.
    let Some(q) = super::quotations::load_quotation(state, id, Utc::now()).await? else {
        return Ok(None);
    };
    let proposals = fetch_proposals(&state.pool, id).await?;
    let supplier_ids: Vec<Uuid> = proposals.iter().map(|p| p.supplier_id).collect();
    let names: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, legal_name, cnpj FROM suppliers WHERE id = ANY($1)")
            .bind(&supplier_ids)
            .fetch_all(&state.pool)
            .await?;
    let supplier_names =
        names.into_iter().map(|(id, name, cnpj)| (id, (name, cnpj))).collect();
    let notified: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE quotation_id = $1 AND kind = 'COTACAO_ABERTA'",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    let os: Option<(String, DateTime<Utc>)> =
        sqlx::query_as("SELECT number, issued_at FROM service_orders WHERE quotation_id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    let ticket: Option<(String, bool, Value, DateTime<Utc>, Option<DateTime<Utc>>)> =
        sqlx::query_as(
            "SELECT file_name, late, divergences, uploaded_at, confirmed_at \
             FROM tickets WHERE quotation_id = $1",
        )
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    Ok(Some(Dossier { q, proposals, supplier_names, notified, os, ticket }))
}

/// R9/R10: the complete JSON dossier. CPF masked — full CPF lives only on the OS.
async fn report_json(
    State(state): State<App>,
    Staff(_claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let Some(d) = load_dossier(&state, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let events = list_events(&state.pool, Some(id)).await?;
    let winner = d.q.awarded_proposal_id.and_then(|w| d.proposals.iter().find(|p| p.id == w));
    let economy = winner.map(|w| compute_economy(d.q.reference_price_cents, w.total_price_cents));
    Ok(Json(json!({
        "generatedAt": Utc::now().to_rfc3339(),
        "quotation": {
            "code": d.q.code, "status": d.q.status,
            "origin": d.q.origin, "destination": d.q.destination,
            "departureAt": d.q.departure_at.to_rfc3339(),
            "referenceFlight": d.q.reference_flight,
            "opensAt": d.q.opens_at.map(|v| v.to_rfc3339()),
            "closesAt": d.q.closes_at.map(|v| v.to_rfc3339()),
            "awardedAt": d.q.awarded_at.map(|v| v.to_rfc3339()),
            "awardJustification": d.q.award_justification,
            "passengerName": d.q.passenger_name,
            "passengerCpfMasked": mask_cpf(&d.q.passenger_cpf),
        },
        "referencePriceCents": d.q.reference_price_cents,
        "notifiedSuppliers": d.notified,
        "proposals": d.proposals.iter().enumerate().map(|(i, p)| {
            let (name, cnpj) = d.supplier_names.get(&p.supplier_id).cloned().unwrap_or_default();
            json!({
                "position": i + 1, "supplier": name, "cnpj": cnpj,
                "totalPriceCents": p.total_price_cents, "flightInfo": p.flight_info,
                "submittedAt": p.submitted_at.to_rfc3339()
            })
        }).collect::<Vec<_>>(),
        "winner": winner.map(|w| {
            let (name, cnpj) = d.supplier_names.get(&w.supplier_id).cloned().unwrap_or_default();
            json!({ "supplier": name, "cnpj": cnpj, "totalPriceCents": w.total_price_cents })
        }),
        "serviceOrder": d.os.as_ref().map(|(number, issued_at)| {
            json!({ "number": number, "issuedAt": issued_at.to_rfc3339() })
        }),
        "ticket": d.ticket.as_ref().map(|(file_name, late, divergences, uploaded_at, confirmed_at)| {
            json!({
                "fileName": file_name, "late": late, "divergences": divergences,
                "uploadedAt": uploaded_at.to_rfc3339(),
                "confirmedAt": confirmed_at.map(|v| v.to_rfc3339())
            })
        }),
        "economy": economy,
        "timeline": events.iter().map(|e| json!({
            "seq": e.seq, "at": e.at, "type": e.event_type, "actorId": e.actor_id, "payload": e.payload
        })).collect::<Vec<_>>(),
    })))
}

/// Printable OS page — staff or the winning supplier (opened via ?token=).
async fn service_order_page(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Html<String>> {
    let Some(d) = load_dossier(&state, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let Some((os_number, issued_at)) = d.os.clone() else {
        return Err(ApiError::NotFound("OS_NAO_EMITIDA"));
    };
    let Some(winner) = d.q.awarded_proposal_id.and_then(|w| d.proposals.iter().find(|p| p.id == w))
    else {
        return Err(ApiError::NotFound("OS_NAO_EMITIDA"));
    };
    match claims.role {
        Role::Admin | Role::Servidor => {}
        Role::Fornecedor => {
            if claims.supplier_id != Some(winner.supplier_id) {
                return Err(ApiError::Forbidden("ACESSO_NEGADO"));
            }
        }
    }
    let (supplier_name, supplier_cnpj) =
        d.supplier_names.get(&winner.supplier_id).cloned().unwrap_or_default();
    let template = OsTemplate {
        number: os_number,
        code: d.q.code.clone(),
        supplier_name,
        supplier_cnpj,
        passenger_name: d.q.passenger_name.clone(),
        passenger_cpf: d.q.passenger_cpf.clone(),
        passenger_sex: d.q.passenger_sex.clone(),
        passenger_birth: d.q.passenger_birth.format("%d/%m/%Y").to_string(),
        origin: d.q.origin.clone(),
        destination: d.q.destination.clone(),
        departure_at: format!("{} (horário de Boa Vista)", fmt_boa_vista(d.q.departure_at)),
        flight_info: winner.flight_info.clone(),
        price: format_brl(winner.total_price_cents),
        issued_at: format!("{} (horário de Boa Vista)", fmt_boa_vista(issued_at)),
    };
    Ok(Html(template.render().map_err(|e| ApiError::Internal(e.to_string()))?))
}

/// Printable dossier page — staff only (SEI attachment, prestação de contas).
async fn report_page(
    State(state): State<App>,
    Staff(_claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Html<String>> {
    let Some(d) = load_dossier(&state, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let events = list_events(&state.pool, Some(id)).await?;
    let audit = verify_chain(&state.pool).await?;
    let winner = d.q.awarded_proposal_id.and_then(|w| d.proposals.iter().find(|p| p.id == w));
    let economy = winner.map(|w| compute_economy(d.q.reference_price_cents, w.total_price_cents));
    let ticket_line = match &d.ticket {
        Some((file_name, late, divergences, _, _)) => format!(
            "{} — {} — divergências: {}",
            file_name,
            if *late { "enviado FORA do prazo de 30 min" } else { "enviado dentro do prazo" },
            divergences
                .as_array()
                .map(|a| {
                    if a.is_empty() {
                        "nenhuma".to_string()
                    } else {
                        a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                    }
                })
                .unwrap_or_else(|| "nenhuma".to_string())
        ),
        None => "Ainda não enviado.".to_string(),
    };
    let template = ReportTemplate {
        code: d.q.code.clone(),
        status: d.q.status.clone(),
        origin: d.q.origin.clone(),
        destination: d.q.destination.clone(),
        passenger_name: d.q.passenger_name.clone(),
        passenger_cpf_masked: mask_cpf(&d.q.passenger_cpf),
        reference_price: format_brl(d.q.reference_price_cents),
        notified: d.notified,
        proposals: d
            .proposals
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (name, cnpj) =
                    d.supplier_names.get(&p.supplier_id).cloned().unwrap_or_default();
                ReportProposal {
                    position: i + 1,
                    supplier: name,
                    cnpj,
                    price: format_brl(p.total_price_cents),
                    flight_info: p.flight_info.clone(),
                    submitted_at: fmt_boa_vista(p.submitted_at),
                }
            })
            .collect(),
        has_economy: economy.is_some(),
        economy_saved: economy.as_ref().map(|e| format_brl(e.saved_cents)).unwrap_or_default(),
        economy_pct: economy.as_ref().map(|e| e.saved_pct.to_string()).unwrap_or_default(),
        os_number: d.os.as_ref().map(|(n, _)| n.clone()).unwrap_or_default(),
        ticket_line,
        audit_ok: audit["ok"] == json!(true),
        // Audit timeline stays UTC on purpose: forensic convention matching the
        // hash-chained 'at' values (intentional exception to the Boa Vista UX rule).
        timeline: events
            .iter()
            .map(|e| ReportEvent { seq: e.seq, at: e.at.clone(), event_type: e.event_type.clone() })
            .collect(),
        generated_at: format!("{} (horário de Boa Vista)", fmt_boa_vista(Utc::now())),
    };
    Ok(Html(template.render().map_err(|e| ApiError::Internal(e.to_string()))?))
}

/// KPI block — maps 1:1 to the edital's Indicativos de Sucesso.
async fn metrics_summary(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    #[derive(sqlx::FromRow)]
    struct AwardedRow {
        id: Uuid,
        reference_price_cents: i64,
        awarded_proposal_id: Option<Uuid>,
    }
    let awarded = sqlx::query_as::<_, AwardedRow>(
        "SELECT id, reference_price_cents, awarded_proposal_id FROM quotations \
         WHERE status IN ('AWARDED','TICKETED','COMPLETED')",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut total_saved: i64 = 0;
    let mut participants: i64 = 0;
    let mut tickets_total: i64 = 0;
    let mut tickets_on_time: i64 = 0;
    for row in &awarded {
        if let Some(winner_id) = row.awarded_proposal_id {
            let price: Option<i64> =
                sqlx::query_scalar("SELECT total_price_cents FROM proposals WHERE id = $1")
                    .bind(winner_id)
                    .fetch_optional(&state.pool)
                    .await?;
            if let Some(price) = price {
                total_saved += row.reference_price_cents - price;
            }
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
            .bind(row.id)
            .fetch_one(&state.pool)
            .await?;
        participants += count;
        let late: Option<bool> =
            sqlx::query_scalar("SELECT late FROM tickets WHERE quotation_id = $1")
                .bind(row.id)
                .fetch_optional(&state.pool)
                .await?;
        if let Some(late) = late {
            tickets_total += 1;
            if !late {
                tickets_on_time += 1;
            }
        }
    }
    let awarded_count = awarded.len() as i64;
    Ok(Json(json!({
        "awardedCount": awarded_count,
        "totalSavedCents": total_saved,
        "avgParticipants": if awarded_count > 0 {
            (participants as f64 / awarded_count as f64 * 10.0).round() / 10.0
        } else { 0.0 },
        "ticketsOnTimePct": if tickets_total > 0 {
            (tickets_on_time as f64 / tickets_total as f64 * 100.0).round()
        } else { 0.0 },
    })))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/audit/verify", get(audit_verify))
        .route("/audit/events", get(audit_events))
        .route("/quotations/{id}/report.json", get(report_json))
        .route("/quotations/{id}/report", get(report_page))
        .route("/quotations/{id}/service-order", get(service_order_page))
        .route("/metrics/summary", get(metrics_summary))
}
