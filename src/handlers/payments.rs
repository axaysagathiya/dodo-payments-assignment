use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use blake3::Hasher;
use reqwest::StatusCode as ReqwestStatusCode;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::Utc;
use std::string::String;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::error::AppError;
use crate::types::{InvoiceState, PaymentStatus, PspResponseStatus, PspToken};

#[derive(Deserialize)]
pub struct PaymentRequest {
    pub card_token: PspToken,
    pub idempotency_key: String,
}

#[derive(Serialize)]
pub struct PaymentResponse {
    pub attempt_id: Uuid,
    pub status: PaymentStatus,
    pub psp_reference: Option<String>,
    pub failure_code: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PspChargeRequest {
    amount_cents: i64,
    card_token: PspToken,
}

#[derive(Serialize, Deserialize)]
struct PspChargeResponse {
    status: PspResponseStatus,
    transaction_id: Option<String>,
    code: Option<String>,
}

pub async fn pay_invoice(
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
    Json(payload): Json<PaymentRequest>,
) -> Result<Json<PaymentResponse>, AppError> {
    info!(
        "Processing payment for invoice {} (idempotency: {})",
        invoice_id, payload.idempotency_key
    );

    // 1. Start a transaction
    let mut tx = state.db.begin().await.map_err(|e| {
        error!(
            "Failed to start transaction for invoice {}: {}",
            invoice_id, e
        );
        AppError::database("Failed to start transaction").with_details(e.to_string())
    })?;

    // 2. Lock the invoice row
    let invoice = sqlx::query!(
        r#"
        SELECT total_amount_cents, state as "state: InvoiceState"
        FROM invoices
        WHERE id = $1
        FOR UPDATE
        "#,
        invoice_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to fetch invoice {} with lock: {}", invoice_id, e);
        AppError::database("Failed to fetch invoice with lock").with_details(e.to_string())
    })?
    .ok_or_else(|| {
        warn!("Invoice {} not found during payment attempt", invoice_id);
        AppError::not_found("Invoice not found")
    })?;

    // 3. Check for ANY active or recent processing attempts on this invoice
    let active_attempts = sqlx::query!(
        r#"
        SELECT id, status as "status: PaymentStatus", idempotency_key, psp_reference, failure_code, updated_at, request_hash
        FROM payment_attempts
        WHERE invoice_id = $1
        "#,
        invoice_id
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to check active attempts for invoice {}: {}", invoice_id, e);
        AppError::database("Failed to check active attempts").with_details(e.to_string())
    })?;

    let mut hasher = Hasher::new();
    hasher.update(payload.card_token.as_bytes());
    let request_hash = hasher.finalize().to_hex().to_string();

    for attempt in active_attempts {
        if attempt.idempotency_key == payload.idempotency_key {
            if attempt.request_hash != request_hash {
                warn!(
                    "Idempotency key {} reused with different payload for invoice {}",
                    payload.idempotency_key, invoice_id
                );
                return Err(AppError::conflict(
                    "Idempotency key reused with different request payload",
                ));
            }
        }

        if attempt.status == PaymentStatus::Succeeded {
            if attempt.idempotency_key == payload.idempotency_key {
                info!(
                    "Returning cached successful response for invoice {} (attempt: {})",
                    invoice_id, attempt.id
                );
                return Ok(Json(PaymentResponse {
                    attempt_id: attempt.id,
                    status: attempt.status,
                    psp_reference: attempt.psp_reference,
                    failure_code: attempt.failure_code,
                }));
            } else {
                warn!(
                    "Invoice {} already paid by another attempt ({}), rejecting request {}",
                    invoice_id, attempt.id, payload.idempotency_key
                );
                return Err(AppError::bad_request(
                    "Invoice has already been paid by someone",
                ));
            }
        }

        if attempt.status == PaymentStatus::Processing {
            let now = Utc::now();
            if attempt.updated_at + Duration::from_secs(60) > now {
                warn!(
                    "Active payment attempt {} already in progress for invoice {}",
                    attempt.id, invoice_id
                );
                return Err(AppError::new(
                    StatusCode::CONFLICT,
                    "PAYMENT_IN_PROGRESS",
                    "A payment for this invoice is already being processed. Please try again in a minute.",
                ));
            } else {
                warn!(
                    "Marking stale payment attempt {} as failed for invoice {}",
                    attempt.id, invoice_id
                );
                sqlx::query!(
                    r#"UPDATE payment_attempts SET status = $1, updated_at = NOW() WHERE id = $2"#,
                    PaymentStatus::Failed as PaymentStatus,
                    attempt.id
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error!("Failed to clear stale attempt {}: {}", attempt.id, e);
                    AppError::database("Failed to clear stale attempt").with_details(e.to_string())
                })?;
            }
        }
    }

    // 4. Upsert the current attempt as "processing"
    let attempt_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO payment_attempts (id, invoice_id, status, idempotency_key, request_hash, updated_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (invoice_id, idempotency_key) 
        DO UPDATE SET status = $3, failure_code = NULL, psp_reference = NULL, updated_at = NOW()
        "#,
        attempt_id,
        invoice_id,
        PaymentStatus::Processing as PaymentStatus,
        payload.idempotency_key,
        request_hash
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to record processing attempt for invoice {}: {}", invoice_id, e);
        AppError::database("Failed to prepare payment attempt").with_details(e.to_string())
    })?;

    // Row lock on 'invoices' is HELD during the PSP call.
    info!(
        "Requesting charge from PSP for invoice {} (amount: {})",
        invoice_id, invoice.total_amount_cents
    );

    // 6. Call Mock PSP
    let psp_base_url =
        std::env::var("MOCK_PSP_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string());
    let psp_charge_url = format!("{}/charge", psp_base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| {
            AppError::internal_error("Failed to build HTTP client").with_details(e.to_string())
        })?;

    let psp_res = client
        .post(&psp_charge_url)
        .json(&PspChargeRequest {
            amount_cents: invoice.total_amount_cents,
            card_token: payload.card_token,
        })
        .send()
        .await;

    let (final_status, psp_ref, fail_code, http_status) = match psp_res {
        Ok(res) => {
            let status_code = res.status();
            info!(
                "PSP responded with {} for invoice {}",
                status_code, invoice_id
            );
            let psp_data: Option<PspChargeResponse> = res.json().await.ok();

            match (status_code, psp_data) {
                (ReqwestStatusCode::OK, Some(data))
                    if data.status == PspResponseStatus::Succeeded =>
                {
                    info!(
                        "PSP charge succeeded for invoice {}: {}",
                        invoice_id,
                        data.transaction_id.as_deref().unwrap_or("no-ref")
                    );
                    (
                        PaymentStatus::Succeeded,
                        data.transaction_id,
                        None,
                        StatusCode::OK,
                    )
                }
                (ReqwestStatusCode::PAYMENT_REQUIRED, Some(data)) => {
                    warn!(
                        "PSP charge declined for invoice {}: {:?}",
                        invoice_id, data.code
                    );
                    (
                        PaymentStatus::Failed,
                        None,
                        data.code,
                        StatusCode::PAYMENT_REQUIRED,
                    )
                }
                (ReqwestStatusCode::GATEWAY_TIMEOUT, Some(data)) => {
                    warn!(
                        "PSP charge timed out for invoice {}: {:?}",
                        invoice_id, data.code
                    );
                    (
                        PaymentStatus::Unknown,
                        None,
                        data.code,
                        StatusCode::GATEWAY_TIMEOUT,
                    )
                }
                (ReqwestStatusCode::INTERNAL_SERVER_ERROR, Some(data)) => {
                    error!(
                        "PSP internal error for invoice {}: {:?}",
                        invoice_id, data.code
                    );
                    (
                        PaymentStatus::Unknown,
                        None,
                        data.code,
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
                _ => {
                    error!("Unknown PSP response for invoice {}", invoice_id);
                    (
                        PaymentStatus::Processing,
                        None,
                        None,
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            }
        }
        Err(e) => {
            error!(
                "Network error calling PSP for invoice {}: {}",
                invoice_id, e
            );
            (
                PaymentStatus::Processing,
                None,
                Some("connection_error".to_string()),
                StatusCode::SERVICE_UNAVAILABLE,
            )
        }
    };

    // 7. Final Update (Inside same transaction)
    let updated_record = sqlx::query!(
        r#"
        UPDATE payment_attempts
        SET status = $1, psp_reference = $2, failure_code = $3, updated_at = NOW()
        WHERE invoice_id = $4 AND idempotency_key = $5
        RETURNING id
        "#,
        final_status as PaymentStatus,
        psp_ref,
        fail_code,
        invoice_id,
        payload.idempotency_key
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        error!(
            "Failed to record final PSP result for invoice {}: {}",
            invoice_id, e
        );
        AppError::database("Failed to update payment result").with_details(e.to_string())
    })?;

    if final_status == PaymentStatus::Succeeded {
        info!("Finalizing invoice {} as PAID", invoice_id);
        sqlx::query!(
            r#"UPDATE invoices SET state = $1 WHERE id = $2"#,
            InvoiceState::Paid as InvoiceState,
            invoice_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Failed to mark invoice {} as paid: {}", invoice_id, e);
            AppError::database("Failed to finalize invoice").with_details(e.to_string())
        })?;

        tx.commit().await.map_err(|e| {
            error!(
                "Failed to commit transaction for invoice {}: {}",
                invoice_id, e
            );
            AppError::database("Failed to commit final transaction").with_details(e.to_string())
        })?;

        Ok(Json(PaymentResponse {
            attempt_id: updated_record.id,
            status: final_status,
            psp_reference: psp_ref,
            failure_code: None,
        }))
    } else {
        tx.commit().await.map_err(|e| {
            error!(
                "Failed to commit failure state for invoice {}: {}",
                invoice_id, e
            );
            AppError::database("Failed to commit failure transaction").with_details(e.to_string())
        })?;

        let code = match final_status {
            PaymentStatus::Processing => "PAYMENT_PROCESSING",
            PaymentStatus::Unknown => "PAYMENT_STATUS_UNKNOWN",
            _ => "PAYMENT_FAILED",
        };

        warn!(
            "Payment {} for invoice {} ended with status: {}",
            updated_record.id, invoice_id, code
        );

        Err(AppError::new(
            http_status,
            code.to_string(),
            format!(
                "Payment result: {} (code: {})",
                final_status,
                fail_code.unwrap_or_default()
            ),
        ))
    }
}
