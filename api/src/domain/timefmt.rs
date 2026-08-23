use chrono::{DateTime, FixedOffset, Utc};

/// Boa Vista is UTC-4 year-round (no DST).
pub fn boa_vista(d: DateTime<Utc>) -> DateTime<FixedOffset> {
    d.with_timezone(&FixedOffset::west_opt(4 * 3600).expect("valid offset"))
}

/// Supplier-facing timestamp copy: dd/mm/YYYY HH:mm (horário de Boa Vista).
pub fn fmt_boa_vista(d: DateTime<Utc>) -> String {
    boa_vista(d).format("%d/%m/%Y %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn formats_in_boa_vista_local() {
        let d = Utc.with_ymd_and_hms(2026, 9, 10, 12, 0, 0).unwrap();
        assert_eq!(fmt_boa_vista(d), "10/09/2026 08:00");
    }
}
