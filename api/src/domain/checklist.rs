use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;

use super::types::DocType;

pub const REQUIRED_DOCS: [DocType; 4] =
    [DocType::ContratoSocial, DocType::CndFederal, DocType::CrfFgts, DocType::Cndt];

#[derive(Debug, PartialEq, Serialize)]
pub struct ChecklistResult {
    pub missing: Vec<String>,
    pub expired: Vec<String>,
    pub ok: bool,
}

/// R1 pre-triage ("IA assistiva" deterministic fallback). Pass docs ordered by upload
/// time ascending — the latest document of each type wins.
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
    let ok = missing.is_empty() && expired.is_empty();
    ChecklistResult { missing, expired, ok }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn reports_missing_and_expired() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();
        let docs = vec![
            (DocType::CndFederal, NaiveDate::from_ymd_opt(2026, 1, 1)),
            (DocType::Cndt, NaiveDate::from_ymd_opt(2027, 1, 1)),
        ];
        let result = checklist(&docs, today);
        assert_eq!(result.missing, vec!["CONTRATO_SOCIAL", "CRF_FGTS"]);
        assert_eq!(result.expired, vec!["CND_FEDERAL"]);
        assert!(!result.ok);
    }

    #[test]
    fn ok_when_all_present_and_valid() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();
        let future = NaiveDate::from_ymd_opt(2027, 12, 31);
        let docs: Vec<_> = REQUIRED_DOCS.iter().map(|t| (*t, future)).collect();
        assert!(checklist(&docs, today).ok);
    }
}
