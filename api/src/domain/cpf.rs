/// LGPD masking for screens/reports: keeps middle 6 digits only.
pub fn mask_cpf(cpf: &str) -> String {
    let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return "***".to_string();
    }
    format!("***.{}.{}-**", &digits[3..6], &digits[6..9])
}

#[cfg(test)]
mod tests {
    use super::mask_cpf;

    #[test]
    fn masks_first_three_and_check_digits() {
        assert_eq!(mask_cpf("123.456.789-09"), "***.456.789-**");
        assert_eq!(mask_cpf("bogus"), "***");
    }
}
