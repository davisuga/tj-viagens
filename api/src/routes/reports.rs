use axum::Router;

use crate::App;

pub fn router() -> Router<App> {
    Router::new()
}
