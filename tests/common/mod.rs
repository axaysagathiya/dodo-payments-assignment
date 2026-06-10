use invoice_payment_system::db;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::process::{Child, Command};
use std::time::Duration;
use uuid::Uuid;

pub struct KillOnDrop(pub Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

pub async fn wait_for_url(url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(url).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("Timeout waiting for service at {}", url);
}

pub struct TestApp {
    pub api_url: String,
    pub api_key: String,
    pub _business_id: String,
    pub customer_id: String,
    pub client: reqwest::Client,
    #[allow(unused)]
    pub pool: PgPool,
    // Guards to keep processes alive during the test
    pub _api_guard: KillOnDrop,
    pub _psp_guard: Option<KillOnDrop>,
}

impl TestApp {
    pub async fn spawn(psp_port: u16) -> Self {
        dotenvy::dotenv().ok();
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
        let pool = db::init_pool().await.expect("Failed to initialize DB pool");

        let api_port = 8080;

        // 1. Start Mock PSP
        let psp_child = Command::new("cargo")
            .args(["run", "--bin", "mock_psp"])
            .env("MOCK_PSP_SERVER_URL", format!("127.0.0.1:{}", psp_port))
            .spawn()
            .expect("Failed to start mock_psp");
        let psp_guard = KillOnDrop(psp_child);
        wait_for_url(&format!("http://127.0.0.1:{}/charge", psp_port)).await;

        // 2. Start API
        let api_child = Command::new("cargo")
            .args(["run", "--bin", "invoice-payment-system"])
            .env("DATABASE_URL", &db_url)
            .env(
                "MOCK_PSP_BASE_URL",
                format!("http://127.0.0.1:{}", psp_port),
            )
            .spawn()
            .expect("Failed to start API");
        let api_guard = KillOnDrop(api_child);
        let api_url = format!("http://127.0.0.1:{}", api_port);
        wait_for_url(&format!("{}/business", api_url)).await;

        let client = reqwest::Client::new();

        // 3. Setup Test Data
        let biz_res = client
            .post(format!("{}/business", api_url))
            .json(
                &json!({"name": "Test Biz", "email": format!("test_{}@test.com", Uuid::new_v4())}),
            )
            .send()
            .await
            .unwrap();
        let biz_data: serde_json::Value = biz_res.json().await.unwrap();
        let api_key = biz_data["api_key"].as_str().unwrap().to_string();
        let _business_id = biz_data["business_id"].as_str().unwrap().to_string();

        let cust_res = client
            .post(format!("{}/customer", api_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({"name": "Test Cust", "email": "test@test.com"}))
            .send()
            .await
            .unwrap();
        let customer_id = cust_res.json::<serde_json::Value>().await.unwrap()["customer_id"]
            .as_str()
            .unwrap()
            .to_string();

        Self {
            api_url,
            api_key,
            _business_id,
            customer_id,
            client,
            pool,
            _api_guard: api_guard,
            _psp_guard: Some(psp_guard),
        }
    }

    pub async fn create_invoice(&self, amount: i64) -> String {
        let res = self.client.post(format!("{}/invoice", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "customer_id": self.customer_id,
                "items": [{ "description": "Test Item", "quantity": 1, "unit_amount_cents": amount }]
            }))
            .send().await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        res.json::<serde_json::Value>().await.unwrap()["invoice_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[allow(unused)]
    pub fn stop_psp(&mut self) {
        self._psp_guard.take();
    }
}
