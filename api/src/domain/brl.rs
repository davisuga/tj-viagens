/// Formats integer cents as pt-BR currency: 123456 -> "R$ 1.234,56".
pub fn format_brl(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    let frac = abs % 100;
    let mut int_str = (abs / 100).to_string();
    let mut grouped = String::new();
    while int_str.len() > 3 {
        let split = int_str.len() - 3;
        grouped = format!(".{}{}", &int_str[split..], grouped);
        int_str.truncate(split);
    }
    format!("{sign}R$ {int_str}{grouped},{frac:02}")
}

#[cfg(test)]
mod tests {
    use super::format_brl;

    #[test]
    fn formats_cents() {
        assert_eq!(format_brl(123456), "R$ 1.234,56");
        assert_eq!(format_brl(89000), "R$ 890,00");
        assert_eq!(format_brl(5), "R$ 0,05");
        assert_eq!(format_brl(185000000), "R$ 1.850.000,00");
    }
}
