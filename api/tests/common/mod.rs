use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tj_viagens_api::{app, config::Config, AppState};

pub struct TestApp {
    pub base: String,
    pub pool: PgPool,
    pub client: reqwest::Client,
}

pub fn test_config() -> Config {
    Config {
        database_url: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://tj:tj@localhost:5433/tjviagens_test".to_string()),
        jwt_secret: "test-secret".to_string(),
        port: 0,
        proposal_window_minutes: 60,
        ticket_window_minutes: 30,
        upload_dir: std::env::temp_dir().join("tj-uploads").to_string_lossy().to_string(),
        web_origin: "http://localhost:5173".to_string(),
    }
}

pub async fn spawn_app() -> TestApp {
    let config = test_config();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("test db must be running (docker compose up -d)");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    sqlx::query(
        "TRUNCATE users, suppliers, supplier_documents, quotations, proposals, \
         service_orders, tickets, notifications, audit_events, counters \
         RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");

    let state = AppState::new(pool.clone(), config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app(state)).await.unwrap();
    });
    TestApp {
        base: format!("http://{addr}"),
        pool,
        client: reqwest::Client::new(),
    }
}

use tj_viagens_api::auth::hash_password;
use uuid::Uuid;

pub async fn create_staff(pool: &PgPool, email: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role) \
         VALUES ($1, $2, 'Servidor SGA', $3, 'SERVIDOR')",
    )
    .bind(id)
    .bind(email)
    .bind(hash_password("demo1234"))
    .execute(pool)
    .await
    .unwrap();
    id
}

pub async fn create_supplier(
    pool: &PgPool,
    cnpj: &str,
    email: &str,
    status: &str,
    legal_name: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO suppliers (id, cnpj, legal_name, contact_email, status) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(cnpj)
    .bind(legal_name)
    .bind(email)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, supplier_id) \
         VALUES ($1, $2, 'Titular', $3, 'FORNECEDOR', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(hash_password("demo1234"))
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    id
}

pub async fn login(app: &TestApp, email: &str) -> String {
    let res = app
        .client
        .post(format!("{}/auth/login", app.base))
        .json(&serde_json::json!({ "email": email, "password": "demo1234" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "login failed for {email}");
    let body: serde_json::Value = res.json().await.unwrap();
    body["token"].as_str().unwrap().to_string()
}

/// Registers a supplier through the HTTP API, uploads all 4 required docs, returns supplier id.
pub async fn register_with_docs(app: &TestApp, cnpj: &str, email: &str, name: &str) -> String {
    let reg: serde_json::Value = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&serde_json::json!({
            "cnpj": cnpj, "legalName": name, "contactEmail": email,
            "userName": "Titular", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let supplier_id = reg["supplierId"].as_str().unwrap().to_string();
    let token = login(app, email).await;
    for doc_type in ["CONTRATO_SOCIAL", "CND_FEDERAL", "CRF_FGTS", "CNDT"] {
        let form = reqwest::multipart::Form::new()
            .text("type", doc_type)
            .text("validUntil", "2027-12-31")
            .part(
                "file",
                reqwest::multipart::Part::bytes(b"%PDF-1.4 fake".to_vec()).file_name("doc.pdf"),
            );
        let res = app
            .client
            .post(format!("{}/suppliers/me/documents", app.base))
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 201, "doc upload failed: {doc_type}");
    }
    supplier_id
}

pub fn quotation_payload() -> serde_json::Value {
    serde_json::json!({
        "passengerName": "Maria da Silva",
        "passengerCpf": "123.456.789-09",
        "passengerSex": "F",
        "passengerBirth": "1985-04-12",
        "origin": "BVB",
        "destination": "BSB",
        "departureAt": "2026-09-10T08:00:00Z",
        "referenceFlight": "LA-4001",
        "referencePriceCents": 185000
    })
}

pub async fn create_open_quotation(app: &TestApp, staff_token: &str) -> String {
    let created: serde_json::Value = app
        .client
        .post(format!("{}/quotations", app.base))
        .bearer_auth(staff_token)
        .json(&quotation_payload())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();
    let opened = app
        .client
        .post(format!("{}/quotations/{id}/open", app.base))
        .bearer_auth(staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(opened.status(), 200, "open failed");
    id
}

pub async fn time_travel_past_close(pool: &PgPool, quotation_id: &str) {
    sqlx::query("UPDATE quotations SET closes_at = now() - interval '1 second' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(quotation_id).unwrap())
        .execute(pool)
        .await
        .unwrap();
}

/// Full path to AWARDED: 2 active suppliers bid, window closes, lowest wins.
/// Returns (quotation_id, winner_email, winner_price_cents).
pub async fn setup_awarded(app: &TestApp, staff_token: &str) -> (String, &'static str, i64) {
    create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    let id = create_open_quotation(app, staff_token).await;
    for (email, price) in [("a@example.com", 152300i64), ("b@example.com", 149900)] {
        let token = login(app, email).await;
        let res = app
            .client
            .post(format!("{}/quotations/{id}/proposals", app.base))
            .bearer_auth(&token)
            .json(&serde_json::json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 201);
    }
    time_travel_past_close(&app.pool, &id).await;
    let ranking: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let winner_proposal = ranking["ranking"][0]["proposalId"].as_str().unwrap();
    let award = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(staff_token)
        .json(&serde_json::json!({ "proposalId": winner_proposal, "justification": "Menor preço" }))
        .send()
        .await
        .unwrap();
    assert_eq!(award.status(), 200);
    (id, "b@example.com", 149900)
}

pub fn ticket_form(passenger: &str, departure: &str, price_cents: i64) -> reqwest::multipart::Form {
    reqwest::multipart::Form::new()
        .text("passengerName", passenger.to_string())
        .text("flightInfo", "G3-1720 08:15")
        .text("departureAt", departure.to_string())
        .text("priceCents", price_cents.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"%PDF-1.4 fake ticket".to_vec()).file_name("eticket.pdf"),
        )
}
