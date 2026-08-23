use axum::Router;

use crate::App;

pub mod auth;
pub mod award;
pub mod proposals;
pub mod quotations;
pub mod reports;
pub mod suppliers;
pub mod tickets;
pub mod views;

pub fn router() -> Router<App> {
    Router::new()
        .merge(auth::router())
        .merge(suppliers::router())
        .merge(quotations::router())
        .merge(proposals::router())
        .merge(award::router())
        .merge(tickets::router())
        .merge(reports::router())
}
