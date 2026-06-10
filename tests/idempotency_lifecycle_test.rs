mod common;

use common::TestApp;
use reqwest::StatusCode;
use serde_json::json;
use tokio::time::Duration;

#[tokio::test]
async fn test_idempotency_validation_without_psp() {
    let mut app = TestApp::spawn(3010).await;
    let invoice_id = app.create_invoice(100).await;

    // 1. Request 1: First Payment (Success)
    let idem_key = "idem_lifecycle_test";
    let payload = json!({
        "card_token": "tok_success",
        "idempotency_key": idem_key
    });

    let res1 = app
        .client
        .post(format!("{}/invoices/{}/pay", app.api_url, invoice_id))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res1.status(),
        StatusCode::OK,
        "First request should succeed"
    );

    // 2. Request 2: Same Body (Cached Response)
    let res2 = app
        .client
        .post(format!("{}/invoices/{}/pay", app.api_url, invoice_id))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res2.status(),
        StatusCode::OK,
        "Second request should return cached 200"
    );

    // 3. SHUT DOWN MOCK PSP
    app.stop_psp();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 4. Request 3: Same Key, Different Body (Payload Mismatch)
    let payload_diff = json!({
        "card_token": "tok_card_declined",
        "idempotency_key": idem_key
    });
    let res3 = app
        .client
        .post(format!("{}/invoices/{}/pay", app.api_url, invoice_id))
        .json(&payload_diff)
        .send()
        .await
        .unwrap();

    // Should fail at idempotency check before hitting PSP
    assert_eq!(
        res3.status(),
        StatusCode::CONFLICT,
        "Third request should return 409 Conflict"
    );

    let body3: serde_json::Value = res3.json().await.unwrap();
    assert!(
        body3["message"].as_str().unwrap().contains("reused"),
        "Error should mention idempotency key reuse"
    );
}
