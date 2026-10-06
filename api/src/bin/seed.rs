use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use tj_viagens_api::audit::{append_audit, AuditInput};
use tj_viagens_api::auth::hash_password;
use tj_viagens_api::config::Config;
use tj_viagens_api::db;
use tj_viagens_api::domain::timefmt::boa_vista_date;
use tj_viagens_api::routes::quotations::next_code;
use tj_viagens_api::routes::suppliers::supplier_eligibility;
use uuid::Uuid;

/// Keeps host and database name visible while dropping the credentials. Seed is
/// run against deployed instances, where whatever it prints lands in deploy logs.
fn redact_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    match rest.split_once('@') {
        Some((_userinfo, host)) => format!("{scheme}://***@{host}"),
        None => url.to_string(),
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await;
    println!("⚠ seed: limpando e repopulando {}", redact_url(&config.database_url));
    sqlx::query(
        "TRUNCATE users, suppliers, supplier_documents, quotations, proposals, \
         service_orders, tickets, notifications, audit_events, counters \
         RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();

    let hash = hash_password("demo1234");
    for (email, name, role) in [
        ("admin@tjrr.jus.br", "Administrador STI", "ADMIN"),
        ("servidor@tjrr.jus.br", "Servidor SGA", "SERVIDOR"),
    ] {
        sqlx::query(
            "INSERT INTO users (id, email, name, password_hash, role) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(email)
        .bind(name)
        .bind(&hash)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    }
    let servidor_id: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'servidor@tjrr.jus.br'")
            .fetch_one(&pool)
            .await
            .unwrap();

    // Certidão validity in days from today. Each supplier shows one real situation
    // from the SGA's day: about to expire, fine, and credenciado-but-lapsed.
    let active: [(&str, &str, &str, [DocSpec; 4]); 3] = [
        (
            "11222333000181",
            "Voa Roraima Turismo LTDA",
            "contato@voaroraima.com.br",
            [("CONTRATO_SOCIAL", None), ("CND_FEDERAL", Some(150)), ("CRF_FGTS", Some(6)), ("CNDT", Some(120))],
        ),
        (
            "11444777000161",
            "Amazônia Viagens LTDA",
            "contato@amazoniaviagens.com.br",
            [("CONTRATO_SOCIAL", None), ("CND_FEDERAL", Some(150)), ("CRF_FGTS", Some(25)), ("CNDT", Some(120))],
        ),
        (
            "12345678000195",
            "Rio Branco Turismo LTDA",
            "contato@riobrancotur.com.br",
            [("CONTRATO_SOCIAL", None), ("CND_FEDERAL", Some(150)), ("CRF_FGTS", Some(-3)), ("CNDT", Some(120))],
        ),
    ];
    let mut supplier_ids = Vec::new();
    for (cnpj, name, email, docs) in &active {
        supplier_ids.push(seed_supplier(&pool, &hash, cnpj, name, email, "ACTIVE", docs).await);
    }
    // PENDING supplier missing CNDT — demos the deterministic checklist pre-triage
    seed_supplier(
        &pool,
        &hash,
        "98765432000198",
        "Monte Roraima Travel LTDA",
        "contato@monteroraima.com.br",
        "PENDING",
        &[("CONTRATO_SOCIAL", None), ("CND_FEDERAL", Some(150)), ("CRF_FGTS", Some(25))],
    )
    .await;

    seed_completed_quotation(&pool, servidor_id, &supplier_ids).await;

    println!("Seed ok. Senha universal: demo1234");
    println!("  Servidor:     servidor@tjrr.jus.br");
    println!("  Fornecedores: contato@voaroraima.com.br (CRF vence em 6 dias) | contato@amazoniaviagens.com.br");
    println!("                contato@riobrancotur.com.br (credenciado, CRF vencida: não pode ser adjudicado)");
    println!("  Pendente:     contato@monteroraima.com.br (falta CNDT — demo do checklist)");
}

/// (document type, validity in days from today in Boa Vista; None = no expiry).
type DocSpec = (&'static str, Option<i64>);

async fn seed_supplier(
    pool: &PgPool,
    hash: &str,
    cnpj: &str,
    name: &str,
    email: &str,
    status: &str,
    docs: &[DocSpec],
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO suppliers (id, cnpj, legal_name, contact_email, status) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(cnpj)
    .bind(name)
    .bind(email)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, supplier_id) \
         VALUES ($1,$2,'Titular',$3,'FORNECEDOR',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(hash)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    let today = boa_vista_date(Utc::now());
    for (doc_type, days) in docs {
        // Uploaded weeks ago: the documents predate the seeded award.
        sqlx::query(
            "INSERT INTO supplier_documents \
             (id, supplier_id, doc_type, file_name, file_path, valid_until, uploaded_at) \
             VALUES ($1,$2,$3,'documento.pdf','seed/documento.pdf',$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(doc_type)
        .bind(days.map(|d| today + Duration::days(d)))
        .bind(Utc::now() - Duration::days(20))
        .execute(pool)
        .await
        .unwrap();
    }
    id
}

/// One COMPLETED quotation ~2h in the past: feeds the KPI cards, the dossier and
/// the audit timeline so the very first demo screen already tells the story.
async fn seed_completed_quotation(pool: &PgPool, servidor_id: Uuid, supplier_ids: &[Uuid]) {
    let q_id = Uuid::new_v4();
    let code = next_code(pool, "COT").await.unwrap();
    let opened = Utc::now() - Duration::hours(2);
    let closed = opened + Duration::hours(1);
    let awarded_at = closed + Duration::minutes(4);
    sqlx::query(
        "INSERT INTO quotations (id, code, status, passenger_name, passenger_cpf, passenger_sex, \
         passenger_birth, origin, destination, departure_at, reference_flight, \
         reference_price_cents, opens_at, closes_at, awarded_at, award_justification, \
         ticket_deadline_at, created_by, sei_process, award_reason) \
         VALUES ($1,$2,'COMPLETED','Maria da Silva','12345678909','F','1985-04-12'::date, \
         'BVB','BSB',$3,'LA-4001',185000,$4,$5,$6,'Menor preço entre as propostas habilitadas.',$7,$8, \
         '0004512-38.2026.8.23.8000','MENOR_PRECO')",
    )
    .bind(q_id)
    .bind(&code)
    .bind(opened + Duration::days(20))
    .bind(opened)
    .bind(closed)
    .bind(awarded_at)
    .bind(awarded_at + Duration::minutes(30))
    .bind(servidor_id)
    .execute(pool)
    .await
    .unwrap();

    let winner_price: i64 = 149900;
    let departure = opened + Duration::days(20);
    // The SGA's everyday trade-off out of Roraima: the cheapest goes via Manaus and
    // takes 8h50, R$ 24 more buys 5h40, the direct flight costs R$ 81 more.
    // (supplier, price, airline, flights, minutes from requested departure, duration, connections)
    let bids: [(Uuid, i64, &str, &str, i64, i32, i32); 3] = [
        (supplier_ids[0], 152300, "LATAM", "LA-3341 / LA-3020", 0, 340, 1),
        (supplier_ids[1], winner_price, "GOL", "G3-1720 / G3-1450", -155, 530, 1),
        (supplier_ids[2], 158000, "GOL", "G3-1601", 290, 195, 0),
    ];
    let mut proposal_ids: Vec<Uuid> = Vec::new();
    let mut winner_proposal = Uuid::nil();
    for (i, (supplier_id, price, airline, flights, offset, duration, connections)) in bids.iter().enumerate() {
        let pid = Uuid::new_v4();
        proposal_ids.push(pid);
        if *price == winner_price {
            winner_proposal = pid;
        }
        sqlx::query(
            "INSERT INTO proposals (id, quotation_id, supplier_id, total_price_cents, flight_info, \
             submitted_at, airline, departs_at, duration_minutes, connections) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(pid)
        .bind(q_id)
        .bind(supplier_id)
        .bind(price)
        .bind(flights)
        .bind(opened + Duration::minutes(10 + i as i64 * 7))
        .bind(airline)
        .bind(departure + Duration::minutes(*offset))
        .bind(duration)
        .bind(connections)
        .execute(pool)
        .await
        .unwrap();
    }
    // Decision snapshot, same shape award.rs writes: ranking order (price asc), with
    // each supplier's eligibility as the servidor saw it.
    let mut ranked: Vec<(Uuid, Uuid, i64)> = bids
        .iter()
        .zip(&proposal_ids)
        .map(|(bid, pid)| (*pid, bid.0, bid.1))
        .collect();
    ranked.sort_by_key(|(_, _, price)| *price);
    let eligibility = supplier_eligibility(pool, supplier_ids).await.unwrap();
    let snapshot = json!({
        "decidedAt": awarded_at.to_rfc3339(),
        "reason": "MENOR_PRECO",
        "baselineProposalId": winner_proposal.to_string(),
        "baselinePriceCents": winner_price,
        "bids": ranked.iter().enumerate().map(|(i, (pid, sid, _))| {
            let e = &eligibility[sid];
            json!({
                "position": i + 1,
                "proposalId": pid.to_string(),
                "supplierId": sid.to_string(),
                "eligible": e.eligible,
                "supplierStatus": e.status,
                "docs": e.checklist.docs,
            })
        }).collect::<Vec<_>>(),
    });
    sqlx::query("UPDATE quotations SET awarded_proposal_id = $1, award_snapshot = $2 WHERE id = $3")
        .bind(winner_proposal)
        .bind(&snapshot)
        .bind(q_id)
        .execute(pool)
        .await
        .unwrap();
    let os_number = next_code(pool, "OS").await.unwrap();
    sqlx::query(
        "INSERT INTO service_orders (id, quotation_id, number, issued_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(q_id)
    .bind(&os_number)
    .bind(awarded_at)
    .execute(pool)
    .await
    .unwrap();
    let ticket_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tickets (id, quotation_id, file_name, file_path, passenger_name, flight_info, \
         departure_at, price_cents, divergences, late, uploaded_at, confirmed_at, confirmed_by) \
         VALUES ($1,$2,'eticket-maria.pdf','seed/eticket-maria.pdf','Maria da Silva','G3-1720 / G3-1450', \
         $3,$4,$5,false,$6,$7,$8)",
    )
    .bind(ticket_id)
    .bind(q_id)
    .bind(opened + Duration::days(20))
    .bind(winner_price)
    .bind(json!([]))
    .bind(awarded_at + Duration::minutes(12))
    .bind(awarded_at + Duration::minutes(20))
    .bind(servidor_id)
    .execute(pool)
    .await
    .unwrap();
    for supplier_id in supplier_ids {
        sqlx::query(
            "INSERT INTO notifications (id, supplier_id, quotation_id, kind, message) \
             VALUES ($1,$2,$3,'COTACAO_ABERTA',$4)",
        )
        .bind(Uuid::new_v4())
        .bind(supplier_id)
        .bind(q_id)
        .bind(format!("Nova cotação {code}: BVB → BSB."))
        .execute(pool)
        .await
        .unwrap();
    }

    // Audit trail in the real event order — verify_chain() must return ok:true after seeding.
    let events: Vec<(&str, &str, String, serde_json::Value)> = vec![
        ("QUOTATION_CREATED", "Quotation", q_id.to_string(), json!({ "code": code, "seiProcess": "0004512-38.2026.8.23.8000" })),
        ("QUOTATION_OPENED", "Quotation", q_id.to_string(), json!({ "code": code, "notified": 3 })),
        ("PROPOSAL_SUBMITTED", "Proposal", proposal_ids[0].to_string(), json!({ "totalPriceCents": 152300 })),
        ("PROPOSAL_SUBMITTED", "Proposal", proposal_ids[1].to_string(), json!({ "totalPriceCents": winner_price })),
        ("PROPOSAL_SUBMITTED", "Proposal", proposal_ids[2].to_string(), json!({ "totalPriceCents": 158000 })),
        ("QUOTATION_CLOSED", "Quotation", q_id.to_string(), json!({})),
        ("QUOTATION_AWARDED", "Quotation", q_id.to_string(), json!({
            "proposalId": winner_proposal.to_string(),
            "totalPriceCents": winner_price,
            "position": 1,
            "lowestPriceCents": winner_price,
            "reason": "MENOR_PRECO",
            "isLowestEligiblePrice": true,
            "lowestEligiblePriceCents": winner_price,
            "justification": "Menor preço entre as propostas habilitadas.",
            "snapshot": snapshot,
        })),
        ("SERVICE_ORDER_ISSUED", "ServiceOrder", os_number.clone(), json!({ "number": os_number })),
        ("TICKET_UPLOADED", "Ticket", ticket_id.to_string(), json!({ "late": false, "divergences": [] })),
        ("TICKET_CONFIRMED", "Ticket", ticket_id.to_string(), json!({})),
    ];
    for (event_type, entity, entity_id, payload) in events {
        append_audit(
            pool,
            AuditInput {
                actor_id: Some(servidor_id),
                actor_role: Some("SERVIDOR"),
                event_type,
                entity,
                entity_id,
                quotation_id: Some(q_id),
                payload,
            },
        )
        .await
        .unwrap();
    }
}
