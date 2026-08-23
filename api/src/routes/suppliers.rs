use std::collections::HashMap;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{append_audit, append_audit_tx, AuditInput};
use crate::auth::{hash_password, AuthUser, Claims, Staff};
use crate::domain::checklist::{checklist, ChecklistResult};
use crate::domain::cnpj::is_valid_cnpj;
use crate::domain::types::{DocType, Role};
use crate::error::{ApiError, ApiResult};
use crate::uploads::save_upload;
use crate::App;

pub fn require_supplier(claims: &Claims) -> Result<Uuid, ApiError> {
    match claims.role {
        Role::Fornecedor => claims.supplier_id.ok_or(ApiError::Forbidden("ACESSO_NEGADO")),
        Role::Admin | Role::Servidor => Err(ApiError::Forbidden("ACESSO_NEGADO")),
    }
}

#[derive(sqlx::FromRow)]
struct DocRow {
    doc_type: String,
    valid_until: Option<NaiveDate>,
}

pub async fn load_checklist(pool: &PgPool, supplier_id: Uuid) -> ApiResult<ChecklistResult> {
    let rows = sqlx::query_as::<_, DocRow>(
        "SELECT doc_type, valid_until FROM supplier_documents \
         WHERE supplier_id = $1 ORDER BY uploaded_at ASC",
    )
    .bind(supplier_id)
    .fetch_all(pool)
    .await?;
    let docs: Vec<(DocType, Option<NaiveDate>)> = rows
        .iter()
        .filter_map(|r| DocType::parse(&r.doc_type).map(|t| (t, r.valid_until)))
        .collect();
    Ok(checklist(&docs, Utc::now().date_naive()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterBody {
    cnpj: String,
    legal_name: String,
    contact_email: String,
    phone: Option<String>,
    user_name: String,
    password: String,
}

async fn register(
    State(state): State<App>,
    Json(body): Json<RegisterBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    if !is_valid_cnpj(&body.cnpj) {
        return Err(ApiError::Unprocessable("CNPJ_INVALIDO"));
    }
    if body.password.chars().count() < 8 {
        return Err(ApiError::Unprocessable("SENHA_CURTA"));
    }
    if body.password.len() > 256 {
        return Err(ApiError::Unprocessable("SENHA_LONGA"));
    }
    let cnpj: String = body.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    let dup_supplier: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM suppliers WHERE cnpj = $1").bind(&cnpj)
            .fetch_optional(&state.pool).await?;
    let dup_user: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = $1").bind(&body.contact_email)
            .fetch_optional(&state.pool).await?;
    if dup_supplier.is_some() || dup_user.is_some() {
        return Err(ApiError::Conflict("JA_CADASTRADO"));
    }
    let supplier_id = Uuid::new_v4();
    let password_hash = hash_password(&body.password);
    // Race guard: two simultaneous registrations can both pass the SELECT checks
    // above — the unique constraints are the source of truth, mapped to 409.
    let inserted: Result<(), sqlx::Error> = async {
        let mut tx = state.pool.begin().await?;
        sqlx::query(
            "INSERT INTO suppliers (id, cnpj, legal_name, contact_email, phone) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(supplier_id).bind(&cnpj).bind(&body.legal_name).bind(&body.contact_email).bind(&body.phone)
        .execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO users (id, email, name, password_hash, role, supplier_id) \
             VALUES ($1,$2,$3,$4,'FORNECEDOR',$5)",
        )
        .bind(Uuid::new_v4()).bind(&body.contact_email).bind(&body.user_name)
        .bind(&password_hash).bind(supplier_id)
        .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    .await;
    if let Err(err) = inserted {
        if err.as_database_error().is_some_and(|db| db.is_unique_violation()) {
            return Err(ApiError::Conflict("JA_CADASTRADO"));
        }
        return Err(err.into());
    }
    append_audit(&state.pool, AuditInput {
        actor_id: None,
        actor_role: None,
        event_type: "SUPPLIER_REGISTERED",
        entity: "Supplier",
        entity_id: supplier_id.to_string(),
        quotation_id: None,
        payload: json!({ "cnpj": cnpj, "legalName": body.legal_name }),
    }).await?;
    Ok((StatusCode::CREATED, Json(json!({ "supplierId": supplier_id }))))
}

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SupplierRow {
    id: Uuid,
    cnpj: String,
    legal_name: String,
    contact_email: String,
    phone: Option<String>,
    status: String,
    status_reason: Option<String>,
}

async fn me(State(state): State<App>, AuthUser(claims): AuthUser) -> ApiResult<Json<Value>> {
    let supplier_id = require_supplier(&claims)?;
    let supplier = sqlx::query_as::<_, SupplierRow>(
        "SELECT id, cnpj, legal_name, contact_email, phone, status, status_reason \
         FROM suppliers WHERE id = $1",
    )
    .bind(supplier_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound("NAO_ENCONTRADO"))?;
    let check = load_checklist(&state.pool, supplier_id).await?;
    Ok(Json(json!({ "supplier": supplier, "checklist": check })))
}

async fn upload_document(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let supplier_id = require_supplier(&claims)?;
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut saved: Option<(String, String)> = None;
    while let Some(field) =
        multipart.next_field().await.map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let original = field.file_name().unwrap_or("upload.bin").to_string();
            let bytes = field.bytes().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            saved = Some(save_upload(&state.config.upload_dir, &original, &bytes).await?);
        } else {
            let value = field.text().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            fields.insert(name, value);
        }
    }
    let doc_type = fields
        .get("type")
        .and_then(|t| DocType::parse(t))
        .ok_or(ApiError::Unprocessable("DOCUMENTO_INVALIDO"))?;
    let Some((file_name, file_path)) = saved else {
        return Err(ApiError::Unprocessable("DOCUMENTO_INVALIDO"));
    };
    let valid_until: Option<NaiveDate> = fields.get("validUntil").and_then(|v| v.parse().ok());
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO supplier_documents (id, supplier_id, doc_type, file_name, file_path, valid_until) \
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id).bind(supplier_id).bind(doc_type.as_str()).bind(&file_name).bind(&file_path).bind(valid_until)
    .execute(&state.pool).await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "DOCUMENT_UPLOADED",
        entity: "SupplierDocument",
        entity_id: id.to_string(),
        quotation_id: None,
        payload: json!({ "docType": doc_type.as_str(), "validUntil": fields.get("validUntil") }),
    }).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "type": doc_type.as_str(), "fileName": file_name }))))
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<String>,
}

async fn list_suppliers(
    State(state): State<App>,
    Staff(_claims): Staff,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let rows = match &query.status {
        Some(status) => sqlx::query_as::<_, SupplierRow>(
            "SELECT id, cnpj, legal_name, contact_email, phone, status, status_reason \
             FROM suppliers WHERE status = $1 ORDER BY created_at ASC",
        )
        .bind(status)
        .fetch_all(&state.pool)
        .await?,
        None => sqlx::query_as::<_, SupplierRow>(
            "SELECT id, cnpj, legal_name, contact_email, phone, status, status_reason \
             FROM suppliers ORDER BY created_at ASC",
        )
        .fetch_all(&state.pool)
        .await?,
    };
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let check = load_checklist(&state.pool, row.id).await?;
        result.push(json!({ "supplier": row, "checklist": check }));
    }
    Ok(Json(json!(result)))
}

#[derive(Deserialize)]
struct DecisionBody {
    decision: String,
    reason: Option<String>,
}

async fn decide(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
    Json(body): Json<DecisionBody>,
) -> ApiResult<Json<Value>> {
    let mut tx = state.pool.begin().await?;
    let current: Option<(String,)> =
        sqlx::query_as("SELECT status FROM suppliers WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((status,)) = current else { return Err(ApiError::NotFound("NAO_ENCONTRADO")) };
    if status != "PENDING" {
        return Err(ApiError::Unprocessable("JA_DECIDIDO"));
    }
    let approve = match body.decision.as_str() {
        "APPROVE" => true,
        "REJECT" => false,
        _ => return Err(ApiError::Unprocessable("DECISAO_INVALIDA")),
    };
    if approve {
        // reads supplier_documents only — no lock conflict with the suppliers row lock
        let check = load_checklist(&state.pool, id).await?;
        if !check.ok {
            return Err(ApiError::UnprocessableWith(
                "CHECKLIST_PENDENTE",
                serde_json::to_value(&check).expect("serialize checklist"),
            ));
        }
    }
    let new_status = if approve { "ACTIVE" } else { "REJECTED" };
    sqlx::query(
        "UPDATE suppliers SET status = $1, status_reason = $2, decided_at = now(), decided_by = $3 \
         WHERE id = $4 AND status = 'PENDING'",
    )
    .bind(new_status).bind(&body.reason).bind(claims.sub).bind(id)
    .execute(&mut *tx)
    .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: if approve { "SUPPLIER_APPROVED" } else { "SUPPLIER_REJECTED" },
        entity: "Supplier",
        entity_id: id.to_string(),
        quotation_id: None,
        payload: json!({ "reason": body.reason }),
    })
    .await?;
    let message = if approve {
        "Credenciamento aprovado. Você já pode participar de cotações.".to_string()
    } else {
        format!("Credenciamento rejeitado: {}", body.reason.as_deref().unwrap_or("sem justificativa"))
    };
    sqlx::query("INSERT INTO notifications (id, supplier_id, kind, message) VALUES ($1,$2,'CREDENCIAMENTO',$3)")
        .bind(Uuid::new_v4()).bind(id).bind(&message)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "id": id, "status": new_status })))
}

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    quotation_id: Option<Uuid>,
    kind: String,
    message: String,
    created_at: chrono::DateTime<Utc>,
}

async fn my_notifications(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
) -> ApiResult<Json<Value>> {
    let supplier_id = require_supplier(&claims)?;
    let rows = sqlx::query_as::<_, NotificationRow>(
        "SELECT id, quotation_id, kind, message, created_at FROM notifications \
         WHERE supplier_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(supplier_id)
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.id, "quotationId": r.quotation_id, "kind": r.kind,
                "message": r.message, "createdAt": r.created_at.to_rfc3339()
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/suppliers/register", post(register))
        .route("/suppliers/me", get(me))
        .route("/suppliers/me/documents", post(upload_document))
        .route("/suppliers", get(list_suppliers))
        .route("/suppliers/{id}/decision", post(decide))
        .route("/notifications", get(my_notifications))
}
