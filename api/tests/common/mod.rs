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
