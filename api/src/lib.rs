pub mod auth;
pub mod audit;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod html;
pub mod routes;
pub mod sse;
pub mod uploads;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::config::Config;

pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub channels: Mutex<HashMap<Uuid, broadcast::Sender<sse::SseMsg>>>,
}

pub type App = Arc<AppState>;

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> App {
        Arc::new(AppState { pool, config, channels: Mutex::new(HashMap::new()) })
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "serverNow": chrono::Utc::now().to_rfc3339() }))
}

async fn time_now() -> Json<serde_json::Value> {
    Json(json!({ "serverNow": chrono::Utc::now().to_rfc3339() }))
}

pub fn app(state: App) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    Router::new()
        .route("/health", get(health))
        .route("/time", get(time_now))
        .merge(routes::router())
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |req: &axum::http::Request<axum::body::Body>| {
                tracing::info_span!("http", method = %req.method(), path = %req.uri().path())
            },
        ))
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(state)
}
