use axum::extract::{Json, Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthenticatedBusiness;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct CreateCustomerResponse {
    pub customer_id: Uuid,
}

#[derive(Serialize)]
pub struct Customer {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

// register the customer to a business, returns the customer id
pub async fn create_customer(
    State(state): State<AppState>,
    auth: AuthenticatedBusiness,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<Json<CreateCustomerResponse>, AppError> {
    let customer_id = Uuid::new_v4();

    sqlx::query(r#"INSERT INTO customers (id, business_id, name, email) VALUES ($1, $2, $3, $4)"#)
        .bind(customer_id)
        .bind(auth.business_id)
        .bind(&payload.name)
        .bind(&payload.email)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::database("Failed to create customer").with_details(e.to_string()))?;

    Ok(Json(CreateCustomerResponse { customer_id }))
}

// fetches all customers belonging to a business
pub async fn fetch_all_customers(
    State(state): State<AppState>,
    auth: AuthenticatedBusiness,
) -> Result<Json<Vec<Customer>>, AppError> {
    let customers = sqlx::query_as!(
        Customer,
        r#"
        SELECT id, name, email
        FROM customers
        WHERE business_id = $1
        "#,
        auth.business_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to fetch customers").with_details(e.to_string()))?;

    Ok(Json(customers))
}

pub async fn get_customer(
    State(state): State<AppState>,
    auth: AuthenticatedBusiness,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<Customer>, AppError> {
    let customer = sqlx::query_as!(
        Customer,
        r#"
        SELECT id, name, email
        FROM customers
        WHERE id = $1 AND business_id = $2
        "#,
        customer_id,
        auth.business_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to fetch customer").with_details(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Customer not found"))?;

    Ok(Json(customer))
}
