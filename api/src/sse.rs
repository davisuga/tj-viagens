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
