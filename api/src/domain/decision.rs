//! Plain-language comparison printed in the dossiê when the award is not the
//! cheapest eligible bid. The SGA asked for exactly this in the report: what the
//! TJ paid above the cheapest option, and what it bought with it.

use super::brl::format_brl;
use super::itinerary::{fmt_connections, fmt_duration};

pub struct BidSummary<'a> {
    pub supplier: &'a str,
    pub price_cents: i64,
    pub duration_minutes: Option<i32>,
    pub connections: Option<i32>,
}

/// One decimal, comma separator: 1.6 -> "1,6".
fn fmt_pct(pct: f64) -> String {
    format!("{pct:.1}").replace('.', ",")
}

pub fn compare_with_baseline(winner: &BidSummary, baseline: &BidSummary) -> String {
    let delta = winner.price_cents - baseline.price_cents;
    let reference = format!(
        "proposta de menor preço habilitada ({}, {})",
        baseline.supplier,
        format_brl(baseline.price_cents)
    );
    let mut parts = vec![if delta == 0 {
        format!("Mesmo preço da {reference}")
    } else {
        let pct = delta as f64 * 100.0 / baseline.price_cents as f64;
        format!("{} (+{}%) acima da {reference}", format_brl(delta), fmt_pct(pct))
    }];
    if let (Some(w), Some(b)) = (winner.duration_minutes, baseline.duration_minutes) {
        let pair = format!("{} contra {}", fmt_duration(w), fmt_duration(b));
        parts.push(if w < b {
            format!("viagem {} mais curta ({pair})", fmt_duration(b - w))
        } else if w > b {
            format!("viagem {} mais longa ({pair})", fmt_duration(w - b))
        } else {
            format!("mesma duração ({})", fmt_duration(w))
        });
    }
    if let (Some(w), Some(b)) = (winner.connections, baseline.connections) {
        let pair = format!("{} contra {}", fmt_connections(w), fmt_connections(b));
        parts.push(if w < b {
            format!("menos conexões ({pair})")
        } else if w > b {
            format!("mais conexões ({pair})")
        } else {
            format!("mesmo número de conexões ({})", fmt_connections(w))
        });
    }
    format!("{}.", parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_paying_more_for_a_shorter_trip() {
        let winner = BidSummary {
            supplier: "Voa Roraima",
            price_cents: 152_300,
            duration_minutes: Some(340),
            connections: Some(1),
        };
        let baseline = BidSummary {
            supplier: "Amazônia Viagens",
            price_cents: 149_900,
            duration_minutes: Some(530),
            connections: Some(1),
        };
        assert_eq!(
            compare_with_baseline(&winner, &baseline),
            "R$ 24,00 (+1,6%) acima da proposta de menor preço habilitada (Amazônia Viagens, \
             R$ 1.499,00); viagem 3h10 mais curta (5h40 contra 8h50); mesmo número de conexões \
             (1 conexão)."
        );
    }

    #[test]
    fn names_a_direct_flight_and_omits_unknown_itinerary() {
        let winner = BidSummary {
            supplier: "Rio Branco",
            price_cents: 158_000,
            duration_minutes: None,
            connections: Some(0),
        };
        let baseline = BidSummary {
            supplier: "Amazônia Viagens",
            price_cents: 149_900,
            duration_minutes: Some(530),
            connections: Some(1),
        };
        assert_eq!(
            compare_with_baseline(&winner, &baseline),
            "R$ 81,00 (+5,4%) acima da proposta de menor preço habilitada (Amazônia Viagens, \
             R$ 1.499,00); menos conexões (direto contra 1 conexão)."
        );
    }

    #[test]
    fn same_price_tie_is_stated_as_such() {
        let bid = BidSummary { supplier: "A", price_cents: 100_000, duration_minutes: None, connections: None };
        assert_eq!(
            compare_with_baseline(&bid, &bid),
            "Mesmo preço da proposta de menor preço habilitada (A, R$ 1.000,00)."
        );
    }
}
