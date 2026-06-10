mod common;

use common::TestApp;
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_psp_failure_no_stuck_state() {
    let app = TestApp::spawn(3011).await;
    let invoice_id = app.create_invoice(100).await;

    // 1. Request 1: PSP Network Error
    println!("Sending network error request (tok_network_error)...");
    let res1 = app.client.post(format!("{}/invoices/{}/pay", app.api_url, invoice_id))
        .json(&json!({
            "card_token": "tok_network_error",
            "idempotency_key": "key_fail_1"
        }))
        .send().await.unwrap();
    assert_eq!(res1.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // 2. Verify invoice is NOT stuck
    let get_inv_res = app.client.get(format!("{}/invoice/{}", app.api_url, invoice_id))
        .header("Authorization", format!("Bearer {}", app.api_key))
        .send().await.unwrap();
    let inv_data: serde_json::Value = get_inv_res.json().await.unwrap();
    assert_eq!(inv_data["state"], "open", "Invoice should still be OPEN after a network error");

    // 3. Request 2: Immediate Success with new key
    println!("Sending successful retry...");
    let res2 = app.client.post(format!("{}/invoices/{}/pay", app.api_url, invoice_id))
        .json(&json!({
            "card_token": "tok_success",
            "idempotency_key": "key_success"
        }))
        .send().await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK, "Invoice should be payable immediately after a non-processing error");

    // 4. Final Check: Invoice is PAID
    let final_inv_res = app.client.get(format!("{}/invoice/{}", app.api_url, invoice_id))
        .header("Authorization", format!("Bearer {}", app.api_key))
        .send().await.unwrap();
    assert_eq!(final_inv_res.json::<serde_json::Value>().await.unwrap()["state"], "paid");
}
