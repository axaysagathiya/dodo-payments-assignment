mod common;

use common::TestApp;
use reqwest::StatusCode;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Barrier;
use uuid::Uuid;

#[tokio::test]
async fn test_concurrent_payments() {
    let app = TestApp::spawn(3012).await;
    let invoice_id = app.create_invoice(1000).await;

    // 1. Fire N concurrent POST /pay requests
    let n = 10;
    let barrier = Arc::new(Barrier::new(n));
    let mut handlers = Vec::new();

    for i in 0..n {
        let client = app.client.clone();
        let api_url = app.api_url.clone();
        let inv_id = invoice_id.clone();
        let b = barrier.clone();
        let idempotency_key = format!("idempotency_{}", i);

        handlers.push(tokio::spawn(async move {
            b.wait().await; // Synchronize start
            client.post(format!("{}/invoices/{}/pay", api_url, inv_id))
                .json(&json!({
                    "card_token": "tok_success",
                    "idempotency_key": idempotency_key
                }))
                .send().await
        }));
    }

    let mut results = Vec::new();
    for h in handlers {
        let res = h.await.unwrap().unwrap();
        results.push(res);
    }

    // 2. Assertions
    let mut success_count = 0;
    let mut conflict_count = 0;
    let mut already_paid_count = 0;

    for res in results {
        match res.status() {
            StatusCode::OK => success_count += 1,
            StatusCode::CONFLICT => conflict_count += 1,
            StatusCode::BAD_REQUEST => already_paid_count += 1,
            _ => panic!("Unexpected status: {}", res.status()),
        }
    }

    assert_eq!(success_count, 1, "Exactly one request should succeed");
    assert_eq!(success_count + conflict_count + already_paid_count, n);

    // 3. Verify final state in DB
    let invoice_id_uuid = Uuid::parse_str(&invoice_id).unwrap();
    let invoice_state: String = sqlx::query_scalar("SELECT state FROM invoices WHERE id = $1")
        .bind(invoice_id_uuid)
        .fetch_one(&app.pool).await.unwrap();
    assert_eq!(invoice_state, "paid");
    
    let success_attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM payment_attempts WHERE invoice_id = $1 AND status = 'succeeded'")
        .bind(invoice_id_uuid)
        .fetch_one(&app.pool).await.unwrap();
    assert_eq!(success_attempts, 1);
}
