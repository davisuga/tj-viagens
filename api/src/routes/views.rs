use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::types::QuotationStatus;

#[derive(sqlx::FromRow, Clone)]
pub struct QuotationRow {
    pub id: Uuid,
    pub code: String,
    pub status: String,
    pub passenger_name: String,
    pub passenger_cpf: String,
    pub passenger_sex: String,
    pub passenger_birth: chrono::NaiveDate,
    pub origin: String,
    pub destination: String,
    pub departure_at: DateTime<Utc>,
    pub return_at: Option<DateTime<Utc>>,
    pub reference_flight: String,
    pub reference_price_cents: i64,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub awarded_proposal_id: Option<Uuid>,
    pub awarded_at: Option<DateTime<Utc>>,
    pub award_justification: Option<String>,
    pub ticket_deadline_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Clone)]
pub struct ProposalRow {
    pub id: Uuid,
    pub quotation_id: Uuid,
    pub supplier_id: Uuid,
    pub total_price_cents: i64,
    pub flight_info: String,
    pub notes: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

/// R4: server clock decides. OPEN past closes_at behaves as CLOSED.
pub fn effective_status(
    status: &str,
    closes_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> QuotationStatus {
    let parsed = QuotationStatus::parse(status).unwrap_or(QuotationStatus::Draft);
    match parsed {
        QuotationStatus::Open => match closes_at {
            Some(deadline) if now >= deadline => QuotationStatus::Closed,
            Some(_) | None => QuotationStatus::Open,
        },
        QuotationStatus::Draft
        | QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => parsed,
    }
}

fn iso(d: Option<DateTime<Utc>>) -> Value {
    match d {
        Some(v) => json!(v.to_rfc3339()),
        None => Value::Null,
    }
}

fn base(q: &QuotationRow, status: QuotationStatus, now: DateTime<Utc>) -> Value {
    json!({
        "id": q.id, "code": q.code, "status": status.as_str(),
        "origin": q.origin, "destination": q.destination,
        "departureAt": q.departure_at.to_rfc3339(), "returnAt": iso(q.return_at),
        "referenceFlight": q.reference_flight,
        "opensAt": iso(q.opens_at), "closesAt": iso(q.closes_at),
        "serverNow": now.to_rfc3339(),
    })
}

fn passenger(q: &QuotationRow) -> Value {
    json!({
        "name": q.passenger_name, "cpf": q.passenger_cpf,
        "sex": q.passenger_sex, "birth": q.passenger_birth.format("%Y-%m-%d").to_string()
    })
}

fn proposal_json(p: &ProposalRow) -> Value {
    json!({
        "id": p.id, "supplierId": p.supplier_id, "totalPriceCents": p.total_price_cents,
        "flightInfo": p.flight_info, "notes": p.notes, "submittedAt": p.submitted_at.to_rfc3339()
    })
}

/// Staff see everything — but while OPEN, proposals collapse to a count (sealed bids, R5).
pub fn staff_view(q: &QuotationRow, proposals: &[ProposalRow], now: DateTime<Utc>) -> Value {
    let status = effective_status(&q.status, q.closes_at, now);
    let mut view = base(q, status, now);
    let obj = view.as_object_mut().expect("base is object");
    obj.insert("passenger".into(), passenger(q));
    obj.insert("referencePriceCents".into(), json!(q.reference_price_cents));
    obj.insert("awardedProposalId".into(), json!(q.awarded_proposal_id));
    obj.insert("awardedAt".into(), iso(q.awarded_at));
    obj.insert("awardJustification".into(), json!(q.award_justification));
    obj.insert("ticketDeadlineAt".into(), iso(q.ticket_deadline_at));
    let proposals_value = match status {
        QuotationStatus::Open => json!({ "count": proposals.len() }),
        QuotationStatus::Draft
        | QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => json!(proposals.iter().map(proposal_json).collect::<Vec<_>>()),
    };
    obj.insert("proposals".into(), proposals_value);
    view
}

/// R2/R5: suppliers never see reference price, rival bids, or passenger PII.
/// The winner gains passenger data + ticket deadline after award (needed to emit the ticket).
pub fn supplier_view(
    q: &QuotationRow,
    proposals: &[ProposalRow],
    supplier_id: Uuid,
    now: DateTime<Utc>,
) -> Value {
    let status = effective_status(&q.status, q.closes_at, now);
    let own = proposals.iter().find(|p| p.supplier_id == supplier_id);
    let is_winner = matches!((own, q.awarded_proposal_id), (Some(p), Some(w)) if p.id == w);
    let mut view = base(q, status, now);
    let obj = view.as_object_mut().expect("base is object");
    obj.insert(
        "myProposal".into(),
        match own {
            Some(p) => proposal_json(p),
            None => Value::Null,
        },
    );
    obj.insert("isWinner".into(), json!(is_winner));
    if is_winner {
        obj.insert("passenger".into(), passenger(q));
        obj.insert("ticketDeadlineAt".into(), iso(q.ticket_deadline_at));
    }
    view
}
