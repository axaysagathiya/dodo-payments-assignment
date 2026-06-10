use axum::{
    Router,
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use uuid::Uuid;

use invoice_payment_system::types::{PspResponseStatus, PspToken};

#[derive(Debug, Deserialize)]
struct ChargeRequest {
    amount_cents: i64,
    card_token: PspToken,
}

#[derive(Debug, Serialize)]
struct ChargeResponse {
    status: PspResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let psp_server_url =
        std::env::var("MOCK_PSP_SERVER_URL").unwrap_or_else(|_| "0.0.0.0:3001".to_string());

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let app = Router::new().route("/charge", post(handle_charge));

    info!("Mock PSP listening on {}", psp_server_url);

    let listener = tokio::net::TcpListener::bind(psp_server_url).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_charge(Json(payload): Json<ChargeRequest>) -> Response {
    info!("Received charge request: {:?}", payload);

    // 1. Validation
    if payload.amount_cents <= 0 {
        warn!("Validation failed: amount_cents must be > 0");
        return (
            StatusCode::BAD_REQUEST,
            Json(ChargeResponse {
                status: PspResponseStatus::Error,
                transaction_id: None,
                code: Some("invalid_amount".to_string()),
            }),
        )
            .into_response();
    }

    // 2. Token Behavior Logic
    match payload.card_token {
        PspToken::Success => {
            let txn_id = format!("txn_{}", Uuid::new_v4());
            let res = ChargeResponse {
                status: PspResponseStatus::Succeeded,
                transaction_id: Some(txn_id),
                code: None,
            };
            info!("Transaction succeeded: {:?}", res);
            Json(res).into_response()
        }

        PspToken::CardDeclined => {
            let res = ChargeResponse {
                status: PspResponseStatus::Failed,
                transaction_id: None,
                code: Some("card_declined".to_string()),
            };
            warn!("Transaction declined: {:?}", res);
            (StatusCode::PAYMENT_REQUIRED, Json(res)).into_response()
        }

        PspToken::InsufficientFunds => {
            let res = ChargeResponse {
                status: PspResponseStatus::Failed,
                transaction_id: None,
                code: Some("insufficient_funds".to_string()),
            };
            warn!("Transaction failed (funds): {:?}", res);
            (StatusCode::PAYMENT_REQUIRED, Json(res)).into_response()
        }

        PspToken::Timeout => {
            info!("Simulating network timeout (20s sleep)...");
            sleep(Duration::from_secs(20)).await;
            let res = ChargeResponse {
                status: PspResponseStatus::Unknown,
                transaction_id: None,
                code: Some("timeout".to_string()),
            };
            (StatusCode::GATEWAY_TIMEOUT, Json(res)).into_response()
        }

        PspToken::NetworkError => {
            let res = ChargeResponse {
                status: PspResponseStatus::Unknown,
                transaction_id: None,
                code: Some("network_error".to_string()),
            };
            tracing::error!("Internal PSP network error: {:?}", res);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(res)).into_response()
        }
    }
}
