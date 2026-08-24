use tj_viagens_api::{app, config::Config, db, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,tower_http=debug".into()),
    ).init();
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await;
    let port = config.port;
    let state = AppState::new(pool, config);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("TJ-Viagens API on http://localhost:{port}");
    axum::serve(listener, app(state)).await.unwrap();
}
