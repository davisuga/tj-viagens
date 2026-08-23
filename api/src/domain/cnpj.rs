/// R1: CNPJ format validation with check digits (punctuation ignored).
pub fn is_valid_cnpj(input: &str) -> bool {
    let digits: Vec<u32> = input.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 14 {
        return false;
    }
    if digits.iter().all(|&d| d == digits[0]) {
        return false;
    }
    let dv = |len: usize| -> u32 {
        let weights: &[u32] = if len == 12 {
            &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
        } else {
            &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
        };
        let sum: u32 = weights.iter().zip(&digits).map(|(w, d)| w * d).sum();
        let m = sum % 11;
        if m < 2 { 0 } else { 11 - m }
    };
    dv(12) == digits[12] && dv(13) == digits[13]
}

#[cfg(test)]
mod tests {
    use super::is_valid_cnpj;

    #[test]
    fn accepts_valid_cnpjs() {
        assert!(is_valid_cnpj("11.222.333/0001-81"));
        assert!(is_valid_cnpj("11444777000161"));
        assert!(is_valid_cnpj("12.345.678/0001-95"));
    }

    #[test]
    fn rejects_invalid_cnpjs() {
        assert!(!is_valid_cnpj("11.222.333/0001-82"));
        assert!(!is_valid_cnpj("11.111.111/1111-11"));
        assert!(!is_valid_cnpj("123"));
    }
}
