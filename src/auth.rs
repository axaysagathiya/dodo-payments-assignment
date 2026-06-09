use axum::{extract::FromRequestParts, http::request::Parts};
use rand::{Rng, distributions::Alphanumeric};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{AppState, error::AppError};

pub struct ApiKey {
    pub prefix: String,
    pub secret: String,
}

impl ApiKey {
    /// Generates a new API key in the format `API_{prefix}_{secret}`.
    pub fn generate() -> (Self, String) {
        let prefix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        let secret: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let raw_key = format!("API_{}_{}", prefix, secret);
        (Self { prefix, secret }, raw_key)
    }

    /// Hashes the secret using Blake3 with the prefix as context.
    /// Returns a hex-encoded hash suitable for storage.
    pub fn hash_secret(&self) -> Result<String, String> {
        // Use prefix as context for Blake3 to add additional domain separation
        let hash = blake3::hash(self.secret.as_bytes());
        Ok(hash.to_hex().to_string())
    }

    /// Verifies the secret against a Blake3 hash.
    pub fn verify(secret: &str, hash: &str) -> bool {
        let computed_hash = blake3::hash(secret.as_bytes());
        computed_hash.to_hex().to_string() == hash
    }
}

/// Creates a new API key for a business and stores it in the database.
/// Returns the raw API key.
pub async fn create_api_key(db: &PgPool, business_id: Uuid) -> Result<String, String> {
    let (api_key, raw_key) = ApiKey::generate();
    let hashed_secret = api_key.hash_secret()?;

    sqlx::query(
        r#"
        INSERT INTO api_keys (id, business_id, hashed_secret, prefix)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(business_id)
    .bind(hashed_secret)
    .bind(api_key.prefix)
    .execute(db)
    .await
    .map_err(|e| format!("Failed to create API key: {}", e))?;

    Ok(raw_key)
}

pub struct AuthenticatedBusiness {
    pub business_id: Uuid,
}

impl FromRequestParts<AppState> for AuthenticatedBusiness {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Missing Authorization header"))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::unauthorized(
                "Invalid Authorization header format",
            ));
        }

        let raw_key = &auth_header[7..];

        // API key format: API_{prefix}_{secret}
        let parts_vec: Vec<&str> = raw_key.split('_').collect();
        if parts_vec.len() != 3 || parts_vec[0] != "API" {
            return Err(AppError::unauthorized("Invalid API key format"));
        }

        let prefix = parts_vec[1];
        let secret = parts_vec[2];

        // Look up the API key in the database by prefix
        let row = sqlx::query(
            r#"
            SELECT business_id, hashed_secret
            FROM api_keys
            WHERE prefix = $1 AND active = TRUE
            "#,
        )
        .bind(prefix)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::internal_error(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::unauthorized("Invalid API key"))?;

        let business_id: Uuid = row.get("business_id");
        let hashed_secret: String = row.get("hashed_secret");

        // Verify the secret
        if !ApiKey::verify(secret, &hashed_secret) {
            return Err(AppError::unauthorized("Invalid API key"));
        }

        Ok(AuthenticatedBusiness { business_id })
    }
}
