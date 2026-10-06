//! Structured itinerary of a bid: what the TJ weighs against price.
//!
//! Duration is captured instead of an arrival time on purpose. Arrival in
//! Brasília is printed in Brasília time (UTC-3) while the system speaks Boa Vista
//! time (UTC-4); asking agencies to convert is how a 1-hour error gets in.

pub const MIN_DURATION_MINUTES: i32 = 20;
pub const MAX_DURATION_MINUTES: i32 = 72 * 60;
pub const MAX_CONNECTIONS: i32 = 5;

pub fn validate(airline: &str, duration_minutes: i32, connections: i32) -> Result<(), &'static str> {
    let airline_len = airline.trim().chars().count();
    if !(2..=40).contains(&airline_len) {
        return Err("COMPANHIA_INVALIDA");
    }
    if !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&duration_minutes) {
        return Err("DURACAO_INVALIDA");
    }
    if !(0..=MAX_CONNECTIONS).contains(&connections) {
        return Err("CONEXOES_INVALIDAS");
    }
    Ok(())
}

/// 340 -> "5h40", 185 -> "3h05", 720 -> "12h".
pub fn fmt_duration(minutes: i32) -> String {
    let (hours, rest) = (minutes / 60, minutes % 60);
    if rest == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h{rest:02}")
    }
}

pub fn fmt_connections(connections: i32) -> String {
    if connections == 0 {
        "direto".to_string()
    } else if connections == 1 {
        "1 conexão".to_string()
    } else {
        format!("{connections} conexões")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_duration_like_a_timetable() {
        assert_eq!(fmt_duration(340), "5h40");
        assert_eq!(fmt_duration(185), "3h05");
        assert_eq!(fmt_duration(720), "12h");
        assert_eq!(fmt_duration(45), "0h45");
    }

    #[test]
    fn formats_connections_in_portuguese() {
        assert_eq!(fmt_connections(0), "direto");
        assert_eq!(fmt_connections(1), "1 conexão");
        assert_eq!(fmt_connections(2), "2 conexões");
    }

    #[test]
    fn validates_itinerary_bounds() {
        assert_eq!(validate("GOL", 340, 1), Ok(()));
        assert_eq!(validate(" ", 340, 1), Err("COMPANHIA_INVALIDA"));
        assert_eq!(validate("GOL", 5, 1), Err("DURACAO_INVALIDA"));
        assert_eq!(validate("GOL", MAX_DURATION_MINUTES + 1, 1), Err("DURACAO_INVALIDA"));
        assert_eq!(validate("GOL", 340, -1), Err("CONEXOES_INVALIDAS"));
        assert_eq!(validate("GOL", 340, MAX_CONNECTIONS + 1), Err("CONEXOES_INVALIDAS"));
    }
}
