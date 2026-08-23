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
