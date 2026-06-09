use std::string::String;
use std::time::Duration;
use axum::extract::{State, Json, Path};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use reqwest::StatusCode as ReqwestStatusCode;
use sqlx::types::chrono::Utc;
use blake3::Hasher;
use crate::{AppState};
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
    // 1. Start a transaction
    let mut tx = state.db.begin().await
        .map_err(|e| AppError::database("Failed to start transaction").with_details(e.to_string()))?;

    // 2. Lock the invoice row for update to synchronize concurrent requests for the same invoice.
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
    .map_err(|e| AppError::database("Failed to fetch invoice with lock").with_details(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Invoice not found"))?;

    if invoice.state == InvoiceState::Paid {
        return Err(AppError::bad_request("Invoice is already paid"));
    }

    // 3. Check for ANY active or recent processing attempts on this invoice
    let active_attempts = sqlx::query!(
        r#"
        SELECT id, status as "status: PaymentStatus", idempotency_key, psp_reference, failure_code, updated_at, request_hash
        FROM payment_attempts
        WHERE invoice_id = $1 AND (status = 'processing' OR status = 'succeeded')
        "#,
        invoice_id
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| AppError::database("Failed to check active attempts").with_details(e.to_string()))?;

    let mut hasher = Hasher::new();

    hasher.update(payload.card_token.as_bytes());
    let request_hash = hasher.finalize().to_hex().to_string();

    for attempt in active_attempts {
        if attempt.idempotency_key == payload.idempotency_key {
            if attempt.request_hash != request_hash {
                return Err(AppError::conflict("Idempotency key reused with different request payload"));
            }
        }

        if attempt.status == PaymentStatus::Succeeded {
            // Invoice is already paid.
            if attempt.idempotency_key == payload.idempotency_key {
                // Same client retrying a success - return the original record
                return Ok(Json(PaymentResponse {
                    attempt_id: attempt.id,
                    status: attempt.status,
                    psp_reference: attempt.psp_reference,
                    failure_code: attempt.failure_code,
                }));
            } else {
                return Err(AppError::bad_request("Invoice has already been paid by someone"));
            }
        }

        if attempt.status == PaymentStatus::Processing {
            // Check if the processing attempt is "fresh" (updated within the last 60 seconds)
            let now = Utc::now();
            
            if attempt.updated_at + Duration::from_secs(60) > now {
                // Another request (or this one) is currently in the 60s window
                return Err(AppError::new(
                    StatusCode::CONFLICT,
                    "PAYMENT_IN_PROGRESS",
                    "A payment for this invoice is already being processed. Please try again in a minute."
                ));
            } else {
                // The attempt is stale (> 60s). We mark it as failed so we can take over.
                tracing::warn!("Marking stale payment attempt {} as failed to allow retry", attempt.id);
                sqlx::query!(
                    r#"UPDATE payment_attempts SET status = $1, updated_at = NOW() WHERE id = $2"#,
                    PaymentStatus::Failed as PaymentStatus,
                    attempt.id
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database("Failed to clear stale attempt").with_details(e.to_string()))?;
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
    .map_err(|e| AppError::database("Failed to prepare payment attempt").with_details(e.to_string()))?;

    // 5. Commit the transaction to release the row lock. 
    // The "processing" status in the DB (plus the 60s freshness) will protect against other concurrent calls.
    tx.commit().await
        .map_err(|e| AppError::database("Failed to commit transaction").with_details(e.to_string()))?;

    // 6. Call Mock PSP
    let client = reqwest::Client::new();
    let psp_res = client
        .post("http://localhost:3001/charge")
        .json(&PspChargeRequest {
            amount_cents: invoice.total_amount_cents,
            card_token: payload.card_token,
        })
        .send()
        .await;

    let (final_status, psp_ref, fail_code, http_status) = match psp_res {
        Ok(res) => {
            let status_code = res.status();
            let psp_data: Option<PspChargeResponse> = res.json().await.ok();

            match (status_code, psp_data) {
                (ReqwestStatusCode::OK, Some(data)) if data.status == PspResponseStatus::Succeeded => {
                    (PaymentStatus::Succeeded, data.transaction_id, None, StatusCode::OK)
                }
                (ReqwestStatusCode::PAYMENT_REQUIRED, Some(data)) => {
                    (PaymentStatus::Failed, None, data.code, StatusCode::PAYMENT_REQUIRED)
                }
                (ReqwestStatusCode::GATEWAY_TIMEOUT, Some(data)) => {
                    (PaymentStatus::Unknown, None, data.code, StatusCode::GATEWAY_TIMEOUT)
                }
                (ReqwestStatusCode::INTERNAL_SERVER_ERROR, Some(data)) => {
                    (PaymentStatus::Unknown, None, data.code, StatusCode::INTERNAL_SERVER_ERROR)
                }
                _ => (PaymentStatus::Processing, None, None, StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => (PaymentStatus::Processing, None, Some("connection_error".to_string()), StatusCode::SERVICE_UNAVAILABLE),
    };

    // 7. Final Update
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
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to update payment result").with_details(e.to_string()))?;

    if final_status == PaymentStatus::Succeeded {
        sqlx::query!(
            r#"UPDATE invoices SET state = $1 WHERE id = $2"#,
            InvoiceState::Paid as InvoiceState,
            invoice_id
        )
        .execute(&state.db)
        .await
        .map_err(|e| AppError::database("Failed to finalize invoice").with_details(e.to_string()))?;

        Ok(Json(PaymentResponse {
            attempt_id: updated_record.id,
            status: final_status,
            psp_reference: psp_ref,
            failure_code: None,
        }))
    } else {
        
        let code = if final_status == PaymentStatus::Processing {
            "PAYMENT_PROCESSING"
        } else if final_status == PaymentStatus::Unknown {
            "PAYMENT_STATUS_UNKNOWN"
        } else {
            "PAYMENT_FAILED"
        };

        Err(AppError::new(
            http_status,
            code.to_string(),
            format!("Payment result: {} (code: {})", final_status, fail_code.unwrap_or_default())
        ))
    }
}