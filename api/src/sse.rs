#[derive(Clone, Debug)]
pub struct SseMsg {
    pub event: String,
    pub data: String,
}
