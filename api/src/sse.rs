use serde_json::Value;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::App;

#[derive(Clone, Debug)]
pub struct SseMsg {
    pub event: String,
    pub data: String,
}

pub fn channel_for(state: &App, quotation_id: Uuid) -> broadcast::Sender<SseMsg> {
    let mut channels = state.channels.lock().expect("channels lock");
    channels.entry(quotation_id).or_insert_with(|| broadcast::channel(64).0).clone()
}

/// R5 discipline: publish only status transitions and proposal COUNTS — never bid values.
pub fn publish(state: &App, quotation_id: Uuid, event: &str, data: Value) {
    let sender = channel_for(state, quotation_id);
    let _ = sender.send(SseMsg { event: event.to_string(), data: data.to_string() });
}

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use futures::stream::Stream;
use futures::StreamExt;
use serde_json::json;

use crate::auth::AuthUser;

/// R4: live countdown/state. Auth via ?token= (EventSource cannot set headers).
/// Emits: hello {serverNow} once, tick {serverNow} every 5s, plus published
/// status/proposal events. Never carries bid values.
async fn events(
    State(state): State<App>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let receiver = channel_for(&state, id).subscribe();
    let hello = futures::stream::once(async {
        Ok(Event::default()
            .event("hello")
            .data(json!({ "serverNow": Utc::now().to_rfc3339() }).to_string()))
    });
    let updates = tokio_stream::wrappers::BroadcastStream::new(receiver).filter_map(|msg| async {
        match msg {
            Ok(m) => Some(Ok(Event::default().event(m.event).data(m.data))),
            Err(_) => None,
        }
    });
    let ticks = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(5),
    ))
    .map(|_| {
        Ok(Event::default()
            .event("tick")
            .data(json!({ "serverNow": Utc::now().to_rfc3339() }).to_string()))
    });
    let stream = hello.chain(futures::stream::select(updates, ticks));
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn router() -> Router<App> {
    Router::new().route("/quotations/{id}/events", get(events))
}
