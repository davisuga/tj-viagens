use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

pub const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// serde_json::Value objects are BTreeMaps by default, so to_string() is key-sorted
/// (canonical) on both write and re-read. Payload discipline: ints/strings/bools/null
/// only — floats would break jsonb round-trip determinism.
pub fn event_hash(prev_hash: &str, core: &Value) -> String {
    let canonical = serde_json::to_string(core).expect("serialize audit core");
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

pub struct AuditInput<'a> {
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<&'a str>,
    pub event_type: &'a str,
    pub entity: &'a str,
    pub entity_id: String,
    pub quotation_id: Option<Uuid>,
    pub payload: Value,
}

fn core_value(at: &str, input: &AuditInput) -> Value {
    json!({
        "at": at,
        "actorId": input.actor_id.map(|u| u.to_string()),
        "actorRole": input.actor_role,
        "type": input.event_type,
        "entity": input.entity,
        "entityId": input.entity_id,
        "quotationId": input.quotation_id.map(|u| u.to_string()),
        "payload": input.payload.clone(),
    })
}

/// Append-only, serialized by a pg advisory lock so the chain never forks.
pub async fn append_audit(pool: &PgPool, input: AuditInput<'_>) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(4242)").execute(&mut *tx).await?;
    let prev_hash: String =
        sqlx::query_scalar("SELECT hash FROM audit_events ORDER BY seq DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or_else(|| GENESIS_HASH.to_string());
    let at = Utc::now().to_rfc3339();
    let core = core_value(&at, &input);
    let hash = event_hash(&prev_hash, &core);
    sqlx::query(
        "INSERT INTO audit_events \
         (at, actor_id, actor_role, event_type, entity, entity_id, quotation_id, payload, prev_hash, hash) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(&at)
    .bind(input.actor_id)
    .bind(input.actor_role)
    .bind(input.event_type)
    .bind(input.entity)
    .bind(&input.entity_id)
    .bind(input.quotation_id)
    .bind(&input.payload)
    .bind(&prev_hash)
    .bind(&hash)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct AuditRow {
    pub seq: i64,
    pub at: String,
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<String>,
    pub event_type: String,
    pub entity: String,
    pub entity_id: String,
    pub quotation_id: Option<Uuid>,
    pub payload: Value,
    pub prev_hash: String,
    pub hash: String,
}

pub async fn list_events(pool: &PgPool, quotation_id: Option<Uuid>) -> ApiResult<Vec<AuditRow>> {
    let rows = match quotation_id {
        Some(qid) => {
            sqlx::query_as::<_, AuditRow>(
                "SELECT seq, at, actor_id, actor_role, event_type, entity, entity_id, \
                 quotation_id, payload, prev_hash, hash \
                 FROM audit_events WHERE quotation_id = $1 ORDER BY seq ASC",
            )
            .bind(qid)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, AuditRow>(
                "SELECT seq, at, actor_id, actor_role, event_type, entity, entity_id, \
                 quotation_id, payload, prev_hash, hash \
                 FROM audit_events ORDER BY seq ASC",
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

pub async fn verify_chain(pool: &PgPool) -> ApiResult<Value> {
    let rows = list_events(pool, None).await?;
    let mut prev = GENESIS_HASH.to_string();
    for row in &rows {
        let core = json!({
            "at": row.at,
            "actorId": row.actor_id.map(|u| u.to_string()),
            "actorRole": row.actor_role,
            "type": row.event_type,
            "entity": row.entity,
            "entityId": row.entity_id,
            "quotationId": row.quotation_id.map(|u| u.to_string()),
            "payload": row.payload.clone(),
        });
        if event_hash(&prev, &core) != row.hash || row.prev_hash != prev {
            return Ok(json!({ "ok": false, "count": rows.len(), "brokenAtSeq": row.seq }));
        }
        prev = row.hash.clone();
    }
    Ok(json!({ "ok": true, "count": rows.len() }))
}
