/// LGPD masking for screens/reports: keeps middle 6 digits only.
pub fn mask_cpf(cpf: &str) -> String {
    let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return "***".to_string();
    }
    format!("***.{}.{}-**", &digits[3..6], &digits[6..9])
}

/// Full CPF display formatting (where full CPF is by design, e.g. the OS).
pub fn format_cpf(cpf: &str) -> String {
    let d: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() != 11 {
        return cpf.to_string();
    }
    format!("{}.{}.{}-{}", &d[0..3], &d[3..6], &d[6..9], &d[9..11])
}

#[cfg(test)]
mod tests {
    use super::{format_cpf, mask_cpf};

    #[test]
    fn masks_first_three_and_check_digits() {
        assert_eq!(mask_cpf("123.456.789-09"), "***.456.789-**");
        assert_eq!(mask_cpf("bogus"), "***");
    }

    #[test]
    fn formats_full_cpf() {
        assert_eq!(format_cpf("12345678909"), "123.456.789-09");
    }
}
