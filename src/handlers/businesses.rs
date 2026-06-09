use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::error::AppError;
use crate::{AppState, auth};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub business_id: Uuid,
    pub api_key: String,
}

/// Registers a new business and creates an API key for it.
pub async fn create_business(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let business_id = Uuid::new_v4();

    info!(
        "Registering new business: {} ({})",
        payload.name, payload.email
    );

    // Insert business
    sqlx::query(r#"INSERT INTO businesses (id, name, email) VALUES ($1, $2, $3)"#)
        .bind(business_id)
        .bind(&payload.name)
        .bind(&payload.email)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert business into DB: {}", e);
            AppError::database("Failed to create business").with_details(e.to_string())
        })?;

    // Create API key and persist
    let raw_key = auth::create_api_key(&state.db, business_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to generate API key for business {}: {}",
                business_id,
                e
            );
            AppError::internal_error("Failed to create API key").with_details(e)
        })?;

    info!(
        "Business {} registered successfully with ID {}",
        payload.name, business_id
    );

    Ok(Json(RegisterResponse {
        business_id,
        api_key: raw_key,
    }))
}
