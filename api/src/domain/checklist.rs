use std::collections::HashMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::types::DocType;

pub const REQUIRED_DOCS: [DocType; 4] =
    [DocType::ContratoSocial, DocType::CndFederal, DocType::CrfFgts, DocType::Cndt];

/// Most certidões last about 30 days (CRF do FGTS, municipal), so a one-week
/// warning is the window an agency actually needs to renew before a quotation.
pub const EXPIRING_WITHIN_DAYS: i64 = 7;

// Deserialize: the award snapshot stores these and the dossiê reads them back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocState {
    Ok,
    Expiring,
    Expired,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocStatus {
    pub doc_type: String,
    /// None when the document has no expiry (contrato social) or is missing.
    pub valid_until: Option<NaiveDate>,
    pub state: DocState,
    /// Days until expiry, negative once expired; None without an expiry date.
    pub days_left: Option<i64>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ChecklistResult {
    pub missing: Vec<String>,
    pub expired: Vec<String>,
    pub ok: bool,
    /// One entry per required document, in REQUIRED_DOCS order.
    pub docs: Vec<DocStatus>,
}

/// R1 pre-triage ("IA assistiva" deterministic fallback). Pass docs ordered by upload
/// time ascending — the latest document of each type wins. `today` is the Boa Vista
/// calendar day (see timefmt::boa_vista_date).
pub fn checklist(docs: &[(DocType, Option<NaiveDate>)], today: NaiveDate) -> ChecklistResult {
    let mut latest: HashMap<&'static str, Option<NaiveDate>> = HashMap::new();
    for (doc_type, valid_until) in docs {
        latest.insert(doc_type.as_str(), *valid_until);
    }
    let missing: Vec<String> = REQUIRED_DOCS
        .iter()
        .filter(|t| !latest.contains_key(t.as_str()))
        .map(|t| t.as_str().to_string())
        .collect();
    let mut expired: Vec<String> = latest
        .iter()
        .filter(|(_, valid)| matches!(valid, Some(d) if *d < today))
        .map(|(k, _)| (*k).to_string())
        .collect();
    expired.sort();
    let docs = REQUIRED_DOCS
        .iter()
        .map(|t| {
            let doc_type = t.as_str().to_string();
            match latest.get(t.as_str()) {
                None => DocStatus { doc_type, valid_until: None, state: DocState::Missing, days_left: None },
                Some(None) => DocStatus { doc_type, valid_until: None, state: DocState::Ok, days_left: None },
                Some(Some(valid_until)) => {
                    let days_left = (*valid_until - today).num_days();
                    let state = if days_left < 0 {
                        DocState::Expired
                    } else if days_left <= EXPIRING_WITHIN_DAYS {
                        DocState::Expiring
                    } else {
                        DocState::Ok
                    };
                    DocStatus { doc_type, valid_until: Some(*valid_until), state, days_left: Some(days_left) }
                }
            }
        })
        .collect();
    let ok = missing.is_empty() && expired.is_empty();
    ChecklistResult { missing, expired, ok, docs }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn reports_missing_and_expired() {
        let today = day(2026, 8, 23);
        let docs = vec![
            (DocType::CndFederal, Some(day(2026, 1, 1))),
            (DocType::Cndt, Some(day(2027, 1, 1))),
        ];
        let result = checklist(&docs, today);
        assert_eq!(result.missing, vec!["CONTRATO_SOCIAL", "CRF_FGTS"]);
        assert_eq!(result.expired, vec!["CND_FEDERAL"]);
        assert!(!result.ok);
    }

    #[test]
    fn ok_when_all_present_and_valid() {
        let today = day(2026, 8, 23);
        let future = Some(day(2027, 12, 31));
        let docs: Vec<_> = REQUIRED_DOCS.iter().map(|t| (*t, future)).collect();
        assert!(checklist(&docs, today).ok);
    }

    #[test]
    fn latest_document_of_a_type_wins() {
        let today = day(2026, 8, 23);
        let docs = vec![
            (DocType::CndFederal, Some(day(2026, 1, 1))), // expired, older upload
            (DocType::CndFederal, Some(day(2027, 1, 1))), // renewal, later upload
        ];
        let result = checklist(&docs, today);
        assert!(result.expired.is_empty(), "renewal must supersede the expired doc");
    }

    #[test]
    fn valid_until_today_is_not_expired() {
        let today = day(2026, 8, 23);
        let docs = vec![(DocType::Cndt, Some(today))];
        assert!(checklist(&docs, today).expired.is_empty());
    }

    #[test]
    fn per_document_status_flags_expiring_within_a_week() {
        let today = day(2026, 10, 6);
        let docs = vec![
            (DocType::ContratoSocial, None),
            (DocType::CndFederal, Some(day(2027, 3, 1))),
            (DocType::CrfFgts, Some(day(2026, 10, 12))),
            (DocType::Cndt, Some(day(2026, 10, 3))),
        ];
        let result = checklist(&docs, today);
        let states: Vec<(String, DocState, Option<i64>)> =
            result.docs.iter().map(|d| (d.doc_type.clone(), d.state, d.days_left)).collect();
        assert_eq!(
            states,
            vec![
                ("CONTRATO_SOCIAL".to_string(), DocState::Ok, None),
                ("CND_FEDERAL".to_string(), DocState::Ok, Some(146)),
                ("CRF_FGTS".to_string(), DocState::Expiring, Some(6)),
                ("CNDT".to_string(), DocState::Expired, Some(-3)),
            ]
        );
        // Expiring is still valid today; only the expired CNDT blocks.
        assert_eq!(result.expired, vec!["CNDT"]);
        assert!(!result.ok);
    }

    #[test]
    fn missing_documents_appear_in_the_per_document_list() {
        let result = checklist(&[], day(2026, 10, 6));
        assert!(result.docs.iter().all(|d| d.state == DocState::Missing));
        assert_eq!(result.docs.len(), REQUIRED_DOCS.len());
    }
}
