use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::audit::{list_events, verify_chain};
use crate::auth::Staff;
use crate::error::ApiResult;
use crate::App;

async fn audit_verify(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    Ok(Json(verify_chain(&state.pool).await?))
}

async fn audit_events(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    let rows = list_events(&state.pool, None).await?;
    let events: Vec<Value> = rows
        .iter()
        .map(|e| {
            json!({
                "seq": e.seq, "at": e.at, "type": e.event_type, "entity": e.entity,
                "entityId": e.entity_id, "actorId": e.actor_id, "payload": e.payload
            })
        })
        .collect();
    Ok(Json(json!(events)))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/audit/verify", get(audit_verify))
        .route("/audit/events", get(audit_events))
}
