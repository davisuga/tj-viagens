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
                payload: json!({ "zeta": n, "flightInfo": "G3-1720 08:15 éão", "alpha": true, "note": null }),
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

    // float payloads are rejected before they can poison the chain
    let float_err = append_audit(
        &app.pool,
        AuditInput {
            actor_id: None,
            actor_role: None,
            event_type: "BAD",
            entity: "X",
            entity_id: "f".to_string(),
            quotation_id: None,
            payload: json!({ "pct": 17.68 }),
        },
    )
    .await;
    assert!(float_err.is_err(), "float payload must be rejected");

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

#[tokio::test]
async fn credenciamento_register_docs_checklist_and_homologation() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;

    let bad = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&json!({
            "cnpj": "11.222.333/0001-82", "legalName": "X", "contactEmail": "x@example.com",
            "userName": "X Y", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 422, "wrong check digit must be rejected");

    let supplier_id =
        common::register_with_docs(&app, "11.222.333/0001-81", "contato@voaroraima.com.br", "Voa Roraima Turismo").await;

    let token = common::login(&app, "contato@voaroraima.com.br").await;
    let me: serde_json::Value = app
        .client
        .get(format!("{}/suppliers/me", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["supplier"]["status"], "PENDING");
    assert_eq!(me["checklist"]["ok"], true);

    // duplicate registration -> 409 (also covers the unique-violation race path)
    let dup = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&json!({
            "cnpj": "11.222.333/0001-81", "legalName": "Clone", "contactEmail": "outro@example.com",
            "userName": "Clone", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(dup.status(), 409);

    // approve without docs must fail for another supplier
    let no_docs = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&json!({
            "cnpj": "11.444.777/0001-61", "legalName": "Sem Docs", "contactEmail": "semdocs@example.com",
            "userName": "Titular", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let refused = app
        .client
        .post(format!("{}/suppliers/{}/decision", app.base, no_docs["supplierId"].as_str().unwrap()))
        .bearer_auth(&staff_token)
        .json(&json!({ "decision": "APPROVE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 422);
    assert_eq!(refused.json::<serde_json::Value>().await.unwrap()["error"], "CHECKLIST_PENDENTE");

    // approve the complete one
    let approved = app
        .client
        .post(format!("{}/suppliers/{supplier_id}/decision", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({ "decision": "APPROVE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(approved.status(), 200);
    assert_eq!(approved.json::<serde_json::Value>().await.unwrap()["status"], "ACTIVE");

    // supplier is notified in the panel
    let notifications: serde_json::Value = app
        .client
        .get(format!("{}/notifications", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(notifications[0]["kind"], "CREDENCIAMENTO");

    // staff-only listing blocked for suppliers
    let denied = app
        .client
        .get(format!("{}/suppliers", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);

    // audit chain stays intact through the whole credenciamento flow
    let audit: serde_json::Value = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(audit["ok"], true);
}
