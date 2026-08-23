use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;
use uuid::Uuid;

use crate::auth::{hash_password, sign_token, verify_password, AuthUser};
use crate::domain::types::Role;
use crate::error::{ApiError, ApiResult};
use crate::App;

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    name: String,
    password_hash: String,
    role: String,
    supplier_id: Option<Uuid>,
}

/// Verified against on the unknown-email path so both failure branches pay the
/// same argon2 cost (prevents account/email enumeration by timing).
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password("dummy-timing-equalizer"))
}

async fn login(State(state): State<App>, Json(body): Json<LoginBody>) -> ApiResult<Json<Value>> {
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, password_hash, role, supplier_id FROM users WHERE email = $1",
    )
    .bind(&body.email)
    .fetch_optional(&state.pool)
    .await?;
    let Some(user) = user else {
        let _ = verify_password(dummy_hash(), &body.password);
        return Err(ApiError::Unauthorized);
    };
    if !verify_password(&user.password_hash, &body.password) {
        return Err(ApiError::Unauthorized);
    }
    let role = Role::parse(&user.role).ok_or_else(|| ApiError::Internal("bad role in db".into()))?;
    let token = sign_token(&state.config.jwt_secret, user.id, &user.name, role, user.supplier_id);
    Ok(Json(json!({
        "token": token,
        "user": { "id": user.id, "name": user.name, "role": role, "supplierId": user.supplier_id }
    })))
}

async fn me(AuthUser(claims): AuthUser) -> Json<Value> {
    Json(json!({
        "sub": claims.sub,
        "name": claims.name,
        "role": claims.role,
        "supplierId": claims.supplier_id
    }))
}

pub fn router() -> Router<App> {
    Router::new().route("/auth/login", post(login)).route("/me", get(me))
}
