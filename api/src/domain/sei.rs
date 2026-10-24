//! SEI process number. Every travel request at the TJ already lives in an SEI
//! process, and every document this system prints is attached to it.
//!
//! House formats vary (CNJ-style `0001234-56.2026.8.23.8000`, shorter internal
//! numbers), so the shape check is loose: digits and the usual separators, with
//! enough digits to be a real number.

const MIN_DIGITS: usize = 6;
const MAX_LEN: usize = 40;

pub fn normalize_sei_process(input: &str) -> Result<String, &'static str> {
    let collapsed = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let allowed = collapsed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '/' | ' '));
    let digits = collapsed.chars().filter(|c| c.is_ascii_digit()).count();
    if !allowed || digits < MIN_DIGITS || collapsed.chars().count() > MAX_LEN {
        return Err("PROCESSO_SEI_INVALIDO");
    }
    Ok(collapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_cnj_style_and_trims() {
        assert_eq!(
            normalize_sei_process("  0001234-56.2026.8.23.8000 "),
            Ok("0001234-56.2026.8.23.8000".to_string())
        );
        assert_eq!(normalize_sei_process("12345/2026"), Ok("12345/2026".to_string()));
    }

    #[test]
    fn rejects_text_and_too_few_digits() {
        assert_eq!(normalize_sei_process("SEI 0001234"), Err("PROCESSO_SEI_INVALIDO"));
        assert_eq!(normalize_sei_process("123-45"), Err("PROCESSO_SEI_INVALIDO"));
        assert_eq!(normalize_sei_process(""), Err("PROCESSO_SEI_INVALIDO"));
        assert_eq!(normalize_sei_process(&"1".repeat(41)), Err("PROCESSO_SEI_INVALIDO"));
    }
}
