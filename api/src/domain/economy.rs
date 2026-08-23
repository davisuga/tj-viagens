use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Economy {
    pub reference_cents: i64,
    pub contracted_cents: i64,
    pub saved_cents: i64,
    pub saved_pct: f64,
}

/// R10: nominal + percentage economy per quotation.
pub fn compute_economy(reference_cents: i64, contracted_cents: i64) -> Economy {
    let saved_cents = reference_cents - contracted_cents;
    let saved_pct = if reference_cents > 0 {
        ((saved_cents as f64 / reference_cents as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };
    Economy { reference_cents, contracted_cents, saved_cents, saved_pct }
}

#[cfg(test)]
mod tests {
    use super::compute_economy;

    #[test]
    fn computes_nominal_and_pct() {
        let e = compute_economy(185000, 152300);
        assert_eq!(e.saved_cents, 32700);
        assert_eq!(e.saved_pct, 17.68);
    }

    #[test]
    fn handles_overrun_with_negative_savings() {
        let e = compute_economy(150000, 165000);
        assert_eq!(e.saved_cents, -15000);
        assert_eq!(e.saved_pct, -10.0);
    }
}
