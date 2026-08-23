use chrono::{DateTime, Utc};
use unicode_normalization::UnicodeNormalization;

pub struct TicketFields<'a> {
    pub passenger_name: &'a str,
    pub departure_at: DateTime<Utc>,
    pub price_cents: i64,
}

fn norm(s: &str) -> String {
    let folded: String = s
        .to_uppercase()
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Boa Vista is UTC-4 year-round (no DST) — compare local calendar days, not UTC.
fn day(d: DateTime<Utc>) -> String {
    let boa_vista = chrono::FixedOffset::west_opt(4 * 3600).expect("valid offset");
    d.with_timezone(&boa_vista).format("%Y-%m-%d").to_string()
}

/// R8: deterministic e-ticket conference — the substitutable "AI extraction" fallback.
pub fn compute_divergences(
    passenger_name: &str,
    departure_at: DateTime<Utc>,
    proposal_price_cents: i64,
    ticket: &TicketFields,
) -> Vec<String> {
    let mut divergences = Vec::new();
    if norm(ticket.passenger_name) != norm(passenger_name) {
        divergences.push("PASSAGEIRO_DIVERGENTE".to_string());
    }
    if ticket.price_cents != proposal_price_cents {
        divergences.push("VALOR_DIVERGENTE".to_string());
    }
    if day(ticket.departure_at) != day(departure_at) {
        divergences.push("DATA_DIVERGENTE".to_string());
    }
    divergences
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn accepts_accent_and_case_variations() {
        let dep = Utc.with_ymd_and_hms(2026, 9, 10, 8, 0, 0).unwrap();
        let ticket = TicketFields { passenger_name: "  MARIA DA SILVA ", departure_at: dep, price_cents: 149900 };
        assert!(compute_divergences("Maria da Silva", dep, 149900, &ticket).is_empty());
    }

    #[test]
    fn flags_wrong_passenger_price_and_date() {
        let dep = Utc.with_ymd_and_hms(2026, 9, 10, 8, 0, 0).unwrap();
        let ticket = TicketFields {
            passenger_name: "João Souza",
            departure_at: Utc.with_ymd_and_hms(2026, 9, 11, 8, 0, 0).unwrap(),
            price_cents: 155000,
        };
        assert_eq!(
            compute_divergences("Maria da Silva", dep, 149900, &ticket),
            vec!["PASSAGEIRO_DIVERGENTE", "VALOR_DIVERGENTE", "DATA_DIVERGENTE"]
        );
    }

    #[test]
    fn compares_days_in_boa_vista_local_time_not_utc() {
        // 23:30 local day 10 (03:30Z day 11) vs 00:15 local day 11 (04:15Z day 11):
        // same UTC date, different Boa Vista date -> must flag.
        let dep = Utc.with_ymd_and_hms(2026, 9, 11, 3, 30, 0).unwrap();
        let ticket = TicketFields {
            passenger_name: "Maria da Silva",
            departure_at: Utc.with_ymd_and_hms(2026, 9, 11, 4, 15, 0).unwrap(),
            price_cents: 149900,
        };
        assert_eq!(compute_divergences("Maria da Silva", dep, 149900, &ticket), vec!["DATA_DIVERGENTE"]);
    }

    #[test]
    fn folds_accents_and_internal_whitespace() {
        let dep = Utc.with_ymd_and_hms(2026, 9, 10, 8, 0, 0).unwrap();
        let ticket = TicketFields {
            passenger_name: "JOSE  ANTONIO CONCEICAO",
            departure_at: dep,
            price_cents: 149900,
        };
        assert!(compute_divergences("José Antônio Conceição", dep, 149900, &ticket).is_empty());
    }
}
