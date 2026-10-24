use chrono::{DateTime, FixedOffset, NaiveDate, Utc};

/// Boa Vista is UTC-4 year-round (no DST).
pub fn boa_vista(d: DateTime<Utc>) -> DateTime<FixedOffset> {
    d.with_timezone(&FixedOffset::west_opt(4 * 3600).expect("valid offset"))
}

/// Calendar day in Boa Vista. Certidão validity is a local date: after 20:00 in
/// Boa Vista the UTC date is already tomorrow, which would expire a document a
/// whole evening early.
pub fn boa_vista_date(d: DateTime<Utc>) -> NaiveDate {
    boa_vista(d).date_naive()
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

    #[test]
    fn local_date_lags_utc_after_20h_in_boa_vista() {
        // 02:00 UTC on the 7th is still 22:00 on the 6th in Boa Vista.
        let d = Utc.with_ymd_and_hms(2026, 10, 7, 2, 0, 0).unwrap();
        assert_eq!(boa_vista_date(d), NaiveDate::from_ymd_opt(2026, 10, 6).unwrap());
    }
}
