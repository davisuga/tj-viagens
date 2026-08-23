use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::types::Role;
use crate::error::ApiError;
use crate::App;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub name: String,
    pub role: Role,
    pub supplier_id: Option<Uuid>,
    pub exp: i64,
}

pub fn sign_token(
    secret: &str,
    sub: Uuid,
    name: &str,
    role: Role,
    supplier_id: Option<Uuid>,
) -> String {
    let claims = Claims {
        sub,
        name: name.to_string(),
        role,
        supplier_id,
        exp: Utc::now().timestamp() + 8 * 3600,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .expect("jwt sign cannot fail with hs256")
}

pub fn verify_token(secret: &str, token: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|data| data.claims)
        .map_err(|_| ApiError::Unauthorized)
}

/// Bearer header first; falls back to ?token= (EventSource and browser-opened
/// printable pages cannot set headers).
fn token_from_parts(parts: &Parts) -> Option<String> {
    if let Some(value) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(t) = s.strip_prefix("Bearer ") {
                return Some(t.to_string());
            }
        }
    }
    let query = parts.uri.query()?;
    query.split('&').find_map(|pair| pair.strip_prefix("token=").map(|t| t.to_string()))
}

pub struct AuthUser(pub Claims);

impl FromRequestParts<App> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        let token = token_from_parts(parts).ok_or(ApiError::Unauthorized)?;
        Ok(AuthUser(verify_token(&state.config.jwt_secret, &token)?))
    }
}

pub struct Staff(pub Claims);

impl FromRequestParts<App> for Staff {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        if claims.role.is_staff() {
            Ok(Staff(claims))
        } else {
            Err(ApiError::Forbidden("ACESSO_NEGADO"))
        }
    }
}

pub fn hash_password(password: &str) -> String {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;
    PasswordHash::new(hash)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}
