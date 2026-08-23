#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub proposal_window_minutes: i64,
    pub ticket_window_minutes: i64,
    pub upload_dir: String,
    pub web_origin: String,
}

fn var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            database_url: var("DATABASE_URL", "postgresql://tj:tj@localhost:5433/tjviagens"),
            jwt_secret: var("JWT_SECRET", "dev-secret-change-me"),
            port: var("PORT", "3001").parse().unwrap_or(3001),
            proposal_window_minutes: var("PROPOSAL_WINDOW_MINUTES", "60").parse().unwrap_or(60),
            ticket_window_minutes: var("TICKET_WINDOW_MINUTES", "30").parse().unwrap_or(30),
            upload_dir: var("UPLOAD_DIR", "uploads"),
            web_origin: var("WEB_ORIGIN", "http://localhost:5173"),
        }
    }
}
