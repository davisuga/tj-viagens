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

#[tokio::test]
async fn supplier_decision_is_atomic_and_single_shot() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let supplier_id =
        common::register_with_docs(&app, "11.222.333/0001-81", "atomic@example.com", "Atomic Tur").await;

    let url = format!("{}/suppliers/{supplier_id}/decision", app.base);
    let (r1, r2) = tokio::join!(
        app.client.post(&url).bearer_auth(&staff_token).json(&json!({ "decision": "APPROVE" })).send(),
        app.client.post(&url).bearer_auth(&staff_token).json(&json!({ "decision": "APPROVE" })).send(),
    );
    let mut statuses = [r1.unwrap().status().as_u16(), r2.unwrap().status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 422], "exactly one decision must win");

    let decision_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE event_type IN ('SUPPLIER_APPROVED','SUPPLIER_REJECTED')",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(decision_events, 1, "exactly one audit row for the decision");

    // traversal-ish filename is neutralized by save_upload
    let token = common::login(&app, "atomic@example.com").await;
    let form = reqwest::multipart::Form::new()
        .text("type", "CND_FEDERAL")
        .text("validUntil", "2027-12-31")
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"%PDF-1.4 fake".to_vec()).file_name("../../evil.pdf"),
        );
    let up = app
        .client
        .post(format!("{}/suppliers/me/documents", app.base))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(up.status(), 201);
    let stored: String = sqlx::query_scalar(
        "SELECT file_path FROM supplier_documents ORDER BY uploaded_at DESC LIMIT 1",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(!stored.contains(".."), "stored path must not contain traversal: {stored}");
    assert!(stored.ends_with("-evil.pdf"), "only the basename survives: {stored}");
}

#[tokio::test]
async fn quotation_open_notifies_active_suppliers_and_redacts_for_suppliers() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    common::create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    common::create_supplier(&app.pool, "12345678000195", "p@example.com", "PENDING", "Pendente Tur").await;

    let id = common::create_open_quotation(&app, &staff_token).await;

    // only the 2 ACTIVE suppliers were notified (R3)
    let notified: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE kind = 'COTACAO_ABERTA'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(notified, 2);

    // staff view: reference price + passenger + proposal count
    let staff_json: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(staff_json["code"], "COT-2026-0001");
    assert_eq!(staff_json["referencePriceCents"], 185000);
    assert_eq!(staff_json["proposals"], json!({ "count": 0 }));

    // supplier view: NO reference price, NO passenger PII, NO rival proposals (R2/R5)
    let supplier_token = common::login(&app, "a@example.com").await;
    let raw = app
        .client
        .get(format!("{}/quotations/{id}", app.base))
        .bearer_auth(&supplier_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!raw.contains("185000"), "reference price leaked: {raw}");
    assert!(!raw.contains("Maria"), "passenger PII leaked: {raw}");
    assert!(!raw.contains("referencePriceCents"));
    let supplier_json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(supplier_json["myProposal"], serde_json::Value::Null);

    // PENDING supplier is blocked entirely
    let pending_token = common::login(&app, "p@example.com").await;
    let denied = app
        .client
        .get(format!("{}/quotations/{id}", app.base))
        .bearer_auth(&pending_token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
}

#[tokio::test]
async fn lazy_close_audits_exactly_once_and_open_is_single_shot() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    let id = common::create_open_quotation(&app, &staff_token).await;
    common::time_travel_past_close(&app.pool, &id).await;

    let url = format!("{}/quotations/{id}", app.base);
    let (r1, r2, r3) = tokio::join!(
        app.client.get(&url).bearer_auth(&staff_token).send(),
        app.client.get(&url).bearer_auth(&staff_token).send(),
        app.client.get(&url).bearer_auth(&staff_token).send(),
    );
    for r in [r1.unwrap(), r2.unwrap(), r3.unwrap()] {
        assert_eq!(r.status(), 200);
    }
    let closed_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE event_type = 'QUOTATION_CLOSED'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(closed_events, 1, "exactly one close event under concurrent reads");
    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "CLOSED");

    // open() is single-shot: reopening a non-DRAFT loses with 422
    let reopen = app
        .client
        .post(format!("{}/quotations/{id}/open", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(reopen.status(), 422);

    // supplier-facing copy is Boa Vista local, never raw UTC
    let msg: String = sqlx::query_scalar(
        "SELECT message FROM notifications WHERE kind = 'COTACAO_ABERTA' LIMIT 1",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(msg.contains("horário de Boa Vista"), "must state the timezone: {msg}");
    assert!(!msg.contains("+00:00"), "no raw UTC in supplier copy: {msg}");
}

#[tokio::test]
async fn proposals_concurrent_bids_replacement_and_window_enforcement() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    common::create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    common::create_supplier(&app.pool, "12345678000195", "c@example.com", "ACTIVE", "Rio Branco Tur").await;
    let id = common::create_open_quotation(&app, &staff_token).await;

    // simultaneous blind bids
    let tokens = [
        common::login(&app, "a@example.com").await,
        common::login(&app, "b@example.com").await,
        common::login(&app, "c@example.com").await,
    ];
    let prices = [152300i64, 149900, 158000];
    let bids = futures::future::join_all(tokens.iter().zip(prices).map(|(token, price)| {
        app.client
            .post(format!("{}/quotations/{id}/proposals", app.base))
            .bearer_auth(token)
            .json(&json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
            .send()
    }))
    .await;
    for bid in bids {
        assert_eq!(bid.unwrap().status(), 201);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 3);

    // every concurrent bid got its audit row atomically
    let bid_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE event_type = 'PROPOSAL_SUBMITTED'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(bid_events, 3);

    // replacement keeps first submitted_at and does not duplicate
    let first: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&tokens[0])
        .json(&json!({ "totalPriceCents": 151000, "flightInfo": "G3-1720 08:15" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count_after, 3);
    assert!(first["submittedAt"].as_str().is_some());

    let replaced_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE event_type = 'PROPOSAL_REPLACED'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(replaced_events, 1, "revision audited as its own event type");

    // absurd price rejected
    let absurd = app
        .client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&tokens[1])
        .json(&json!({ "totalPriceCents": 2_000_000_000i64, "flightInfo": "G3-1720" }))
        .send()
        .await
        .unwrap();
    assert_eq!(absurd.status(), 422);

    // window enforcement: server clock says no (R4)
    common::time_travel_past_close(&app.pool, &id).await;
    let late = app
        .client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&tokens[0])
        .json(&json!({ "totalPriceCents": 140000, "flightInfo": "G3-1720 08:15" }))
        .send()
        .await
        .unwrap();
    assert_eq!(late.status(), 422);
    assert_eq!(late.json::<serde_json::Value>().await.unwrap()["error"], "COTACAO_FECHADA");
    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "CLOSED", "lazy close must persist");

    // audit chain stays intact through concurrent atomic appends
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

#[tokio::test]
async fn ranking_orders_lowest_first_with_tiebreak_then_award_starts_ticket_window() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    common::create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    let c_id = common::create_supplier(&app.pool, "12345678000195", "c@example.com", "ACTIVE", "Rio Branco Tur").await;
    let id = common::create_open_quotation(&app, &staff_token).await;

    for (email, price) in [("a@example.com", 152300i64), ("b@example.com", 149900), ("c@example.com", 149900)] {
        let token = common::login(&app, email).await;
        let res = app
            .client
            .post(format!("{}/quotations/{id}/proposals", app.base))
            .bearer_auth(&token)
            .json(&json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 201);
    }
    // deterministic tie-break: c submitted a minute earlier than b
    sqlx::query(
        "UPDATE proposals SET submitted_at = now() - interval '1 minute' \
         WHERE quotation_id = $1 AND supplier_id = $2",
    )
    .bind(uuid::Uuid::parse_str(&id).unwrap())
    .bind(c_id)
    .execute(&app.pool)
    .await
    .unwrap();

    // ranking refused while open
    let early = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(early.status(), 422);

    common::time_travel_past_close(&app.pool, &id).await;
    let ranking: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let names: Vec<&str> = ranking["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["supplier"]["legalName"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Rio Branco Tur", "Amazônia Viagens", "Voa Roraima"]);
    assert_eq!(ranking["ranking"][0]["deltaFromReferenceCents"], 149900 - 185000);

    let winner_proposal = ranking["ranking"][0]["proposalId"].as_str().unwrap();
    let before = chrono::Utc::now();
    let award: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({ "proposalId": winner_proposal, "justification": "Menor preço e conformidade" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(award["serviceOrder"]["number"], "OS-2026-0001");
    let deadline: chrono::DateTime<chrono::Utc> =
        award["ticketDeadlineAt"].as_str().unwrap().parse().unwrap();
    let minutes = (deadline - before).num_minutes();
    assert!((29..=31).contains(&minutes), "ticket window must be ~30 min, got {minutes}");

    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "AWARDED");

    // single-shot: a second award (double-click) loses with 422, exactly one audit row
    let again = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({ "proposalId": winner_proposal, "justification": "Menor preço e conformidade" }))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 422);
    let award_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE event_type = 'QUOTATION_AWARDED'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(award_events, 1);
    let award_payload: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM audit_events WHERE event_type = 'QUOTATION_AWARDED'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(award_payload["position"], 1, "dossier shows the lowest bid won");
    assert_eq!(award_payload["lowestPriceCents"], 149900);

    // winner notification copy is Boa Vista local, never raw UTC
    let msg: String = sqlx::query_scalar(
        "SELECT message FROM notifications WHERE kind = 'VENCEDORA' LIMIT 1",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(msg.contains("horário de Boa Vista"), "must state the timezone: {msg}");
    assert!(!msg.contains("+00:00"), "no raw UTC in supplier copy: {msg}");
}

#[tokio::test]
async fn ticket_upload_divergences_late_flag_and_confirmation() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let (id, winner_email, winner_price) = common::setup_awarded(&app, &staff_token).await;

    // non-winner blocked
    let loser_token = common::login(&app, "a@example.com").await;
    let denied = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&loser_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", winner_price))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);

    let winner_token = common::login(&app, winner_email).await;

    // absurd price on the ticket is rejected before touching the DB
    let absurd = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", 2_000_000_000))
        .send()
        .await
        .unwrap();
    assert_eq!(absurd.status(), 422);
    let ticket_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tickets WHERE quotation_id = $1")
            .bind(uuid::Uuid::parse_str(&id).unwrap())
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(ticket_count_before, 0, "rejected ticket must not be inserted");

    // winner uploads clean ticket in time
    let clean: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("MARIA DA SILVA", "2026-09-10T08:00:00Z", winner_price))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(clean["late"], false);
    assert_eq!(clean["divergences"], json!([]));

    // staff confirms -> COMPLETED
    let confirm = app
        .client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(confirm.status(), 200);
    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "COMPLETED");

    // double-confirm loses with 422; single audit row each for upload + confirm
    let again = app
        .client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 422);
    for event in ["TICKET_UPLOADED", "TICKET_CONFIRMED"] {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE event_type = $1")
                .bind(event)
                .fetch_one(&app.pool)
                .await
                .unwrap();
        assert_eq!(n, 1, "{event} must appear exactly once");
    }
}

#[tokio::test]
async fn ticket_late_and_divergent_price_are_flagged_not_rejected() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let (id, winner_email, _) = common::setup_awarded(&app, &staff_token).await;
    sqlx::query("UPDATE quotations SET ticket_deadline_at = now() - interval '1 second' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&app.pool)
        .await
        .unwrap();
    let winner_token = common::login(&app, winner_email).await;
    let flagged: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", 155000))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(flagged["late"], true);
    assert_eq!(flagged["divergences"], json!(["VALOR_DIVERGENTE"]));
}

#[tokio::test]
async fn report_metrics_and_printable_pages_after_completed_flow() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let (id, winner_email, winner_price) = common::setup_awarded(&app, &staff_token).await;
    let winner_token = common::login(&app, winner_email).await;
    app.client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", winner_price))
        .send()
        .await
        .unwrap();
    app.client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();

    let report: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/report.json", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["quotation"]["status"], "COMPLETED");
    assert_eq!(report["economy"]["saved_cents"], 35100);
    assert_eq!(report["economy"]["saved_pct"], 18.97);
    assert_eq!(report["notifiedSuppliers"], 2);
    assert_eq!(report["quotation"]["passengerCpfMasked"], "***.456.789-**");
    assert_eq!(report["serviceOrder"]["number"], "OS-2026-0001");
    assert!(report["timeline"].as_array().unwrap().len() >= 6);

    // printable OS: staff ok, winner ok (via ?token=), loser 403
    let os_staff = app
        .client
        .get(format!("{}/quotations/{id}/service-order", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(os_staff.status(), 200);
    let os_html = os_staff.text().await.unwrap();
    assert!(os_html.contains("OS-2026-0001"));
    assert!(os_html.contains("Maria da Silva"));

    let os_winner = app
        .client
        .get(format!("{}/quotations/{id}/service-order?token={winner_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(os_winner.status(), 200);

    let loser_token = common::login(&app, "a@example.com").await;
    let os_loser = app
        .client
        .get(format!("{}/quotations/{id}/service-order?token={loser_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(os_loser.status(), 403);

    // printable report page renders with economy + audit badge
    let report_page = app
        .client
        .get(format!("{}/quotations/{id}/report", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(report_page.contains("Economia obtida"));
    assert!(report_page.contains("Cadeia de hashes íntegra"));

    let metrics: serde_json::Value = app
        .client
        .get(format!("{}/metrics/summary", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metrics["awardedCount"], 1);
    assert_eq!(metrics["totalSavedCents"], 35100);
    assert_eq!(metrics["avgParticipants"], 2.0);
    assert_eq!(metrics["ticketsOnTimePct"], 100.0);

    // printable pages render Boa Vista local time
    let report_page_html = app
        .client
        .get(format!("{}/quotations/{id}/report", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(report_page_html.contains("horário de Boa Vista"));

    // dossier honors the lazy close (R4): a lapsed-OPEN quotation reports CLOSED
    let stale_id = common::create_open_quotation(&app, &staff_token).await;
    common::time_travel_past_close(&app.pool, &stale_id).await;
    let stale_report: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{stale_id}/report.json", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stale_report["quotation"]["status"], "CLOSED");
}

#[tokio::test]
async fn sse_stream_delivers_hello_then_proposal_count() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    let id = common::create_open_quotation(&app, &staff_token).await;

    let res = app
        .client
        .get(format!("{}/quotations/{id}/events?token={staff_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(res.headers()["content-type"].to_str().unwrap().contains("text/event-stream"));

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    use futures::StreamExt;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            buffer.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
            if buffer.contains("event: hello") {
                break;
            }
        }
    })
    .await
    .expect("hello event within 5s");

    // a bid publishes a count-only event
    let supplier_token = common::login(&app, "a@example.com").await;
    app.client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&supplier_token)
        .json(&json!({ "totalPriceCents": 150000, "flightInfo": "G3-1720" }))
        .send()
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            buffer.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
            if buffer.contains("\"count\":1") {
                break;
            }
        }
    })
    .await
    .expect("proposal count event within 5s");
    assert!(!buffer.contains("150000"), "SSE must never leak bid values");

    // no token -> 401
    let anon = app
        .client
        .get(format!("{}/quotations/{id}/events", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(anon.status(), 401);

    // unknown quotation id -> 404, no channel created
    let ghost = uuid::Uuid::new_v4();
    let not_found = app
        .client
        .get(format!("{}/quotations/{ghost}/events?token={staff_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(not_found.status(), 404);
}

#[tokio::test]
async fn full_flow_credenciamento_to_report_through_http_only() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;

    // 1. Credenciamento via API: register + 4 docs + homologation, three suppliers
    let companies = [
        ("11.222.333/0001-81", "voa@example.com", "Voa Roraima", 152300i64),
        ("11.444.777/0001-61", "ama@example.com", "Amazônia Viagens", 149900),
        ("12.345.678/0001-95", "rio@example.com", "Rio Branco Tur", 158000),
    ];
    for (cnpj, email, name, _) in &companies {
        let supplier_id = common::register_with_docs(&app, cnpj, email, name).await;
        let approved = app
            .client
            .post(format!("{}/suppliers/{supplier_id}/decision", app.base))
            .bearer_auth(&staff_token)
            .json(&json!({ "decision": "APPROVE" }))
            .send()
            .await
            .unwrap();
        assert_eq!(approved.status(), 200);
    }

    // 2. Demand + simultaneous notification
    let id = common::create_open_quotation(&app, &staff_token).await;
    let notified: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE kind = 'COTACAO_ABERTA'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(notified, 3);

    // 3. Concurrent blind bids
    let bids = futures::future::join_all(companies.iter().map(|(_, email, _, price)| {
        let app_ref = &app;
        let id_ref = &id;
        async move {
            let token = common::login(app_ref, email).await;
            app_ref
                .client
                .post(format!("{}/quotations/{id_ref}/proposals", app_ref.base))
                .bearer_auth(&token)
                .json(&json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
                .send()
                .await
                .unwrap()
        }
    }))
    .await;
    for bid in bids {
        assert_eq!(bid.status(), 201);
    }

    // 4. Close + ranking lowest-first
    common::time_travel_past_close(&app.pool, &id).await;
    let ranking: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let prices: Vec<i64> = ranking["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["totalPriceCents"].as_i64().unwrap())
        .collect();
    assert_eq!(prices, vec![149900, 152300, 158000]);

    // 5. Award + OS
    let award: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({
            "proposalId": ranking["ranking"][0]["proposalId"],
            "justification": "Menor preço entre as propostas válidas"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(award["serviceOrder"]["number"], "OS-2026-0001");

    // 6. Winner e-ticket + confirmation
    let winner_token = common::login(&app, "ama@example.com").await;
    let ticket: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", 149900))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ticket["divergences"], json!([]));
    app.client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();

    // 7. Dossier + audit integrity + KPIs
    let report: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/report.json", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["quotation"]["status"], "COMPLETED");
    assert_eq!(report["economy"]["saved_cents"], 35100);
    assert_eq!(report["notifiedSuppliers"], 3);

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

    let metrics: serde_json::Value = app
        .client
        .get(format!("{}/metrics/summary", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metrics["awardedCount"], 1);
}
