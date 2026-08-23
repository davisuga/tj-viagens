mod common;

use common::spawn_app;

#[tokio::test]
async fn health_reports_server_time() {
    let app = spawn_app().await;
    let res = app.client.get(format!("{}/health", app.base)).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["serverNow"].as_str().unwrap().contains('T'));
}

#[tokio::test]
async fn login_rbac_and_query_token() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima")
        .await;

    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let me: serde_json::Value = app
        .client
        .get(format!("{}/me", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["role"], "SERVIDOR");

    let wrong = app
        .client
        .post(format!("{}/auth/login", app.base))
        .json(&serde_json::json!({ "email": "servidor@tjrr.jus.br", "password": "nope" }))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), 401);

    let anon = app.client.get(format!("{}/me", app.base)).send().await.unwrap();
    assert_eq!(anon.status(), 401);

    let via_query = app
        .client
        .get(format!("{}/me?token={staff_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(via_query.status(), 200, "query-param token must work (SSE + printable pages)");
}

use serde_json::json;
use tj_viagens_api::audit::{append_audit, AuditInput};

#[tokio::test]
async fn audit_chain_appends_verifies_and_detects_tampering() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima")
        .await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;

    for n in [1i64, 2] {
        append_audit(
            &app.pool,
            AuditInput {
                actor_id: None,
                actor_role: None,
                event_type: "TEST_EVENT",
                entity: "X",
                entity_id: n.to_string(),
                quotation_id: None,
                payload: json!({ "n": n }),
            },
        )
        .await
        .unwrap();
    }

    let ok: serde_json::Value = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ok, json!({ "ok": true, "count": 2 }));

    sqlx::query("UPDATE audit_events SET payload = '{\"n\": 999}'::jsonb WHERE seq = 2")
        .execute(&app.pool)
        .await
        .unwrap();
    let broken: serde_json::Value = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(broken["ok"], false);
    assert_eq!(broken["brokenAtSeq"], 2);

    let supplier_token = common::login(&app, "a@example.com").await;
    let denied = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&supplier_token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
}
