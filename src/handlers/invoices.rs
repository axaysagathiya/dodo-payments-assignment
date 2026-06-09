use axum::extract::{State, Json, Path, Query};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState};
use crate::error::AppError;
use crate::auth::AuthenticatedBusiness;
use crate::types::InvoiceState;

#[derive(Deserialize)]
pub struct InvoiceFilter {
    pub status: Option<InvoiceState>,
}

#[derive(Serialize)]
pub struct InvoiceSummary {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub total_amount_cents: i64,
    pub state: InvoiceState,
}

#[derive(Deserialize)]
pub struct InvoiceItemRequest {
    pub description: String,
    pub quantity: i32,
    pub unit_amount_cents: i64,
}

#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    pub customer_id: Uuid,
    pub items: Vec<InvoiceItemRequest>,
}

#[derive(Serialize)]
pub struct CreateInvoiceResponse {
    pub invoice_id: Uuid,
}

#[derive(Serialize)]
pub struct InvoiceItemResponse {
    pub description: String,
    pub quantity: i32,
    pub unit_amount_cents: i64,
}

#[derive(Serialize)]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub total_amount_cents: i64,
    pub state: InvoiceState,
    pub items: Vec<InvoiceItemResponse>,
}

pub async fn create_invoice(
    State(state): State<AppState>,
    auth: AuthenticatedBusiness,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<CreateInvoiceResponse>, AppError> {
    // Verify customer belongs to this business
    let customer = sqlx::query(
        r#"SELECT id FROM customers WHERE id = $1 AND business_id = $2"#
    )
    .bind(payload.customer_id)
    .bind(auth.business_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to query customer").with_details(e.to_string()))?;

    if customer.is_none() {
        return Err(AppError::bad_request("Customer not found or does not belong to the business"));
    }

    let invoice_id = Uuid::new_v4();
    let total_amount_cents: i64 = payload.items.iter().map(|it| it.quantity as i64 * it.unit_amount_cents).sum();

    sqlx::query!(
        r#"INSERT INTO invoices (id, business_id, customer_id, total_amount_cents, state) VALUES ($1, $2, $3, $4, $5)"#,
        invoice_id,
        auth.business_id,
        payload.customer_id,
        total_amount_cents,
        InvoiceState::Open as InvoiceState
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to create invoice").with_details(e.to_string()))?;

    // Insert items
    for item in payload.items.iter() {
        let item_id = Uuid::new_v4();
        sqlx::query!(
            r#"INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_amount_cents) VALUES ($1, $2, $3, $4, $5)"#,
            item_id,
            invoice_id,
            &item.description,
            item.quantity,
            item.unit_amount_cents
        )
        .execute(&state.db)
        .await
        .map_err(|e| AppError::database("Failed to insert invoice item").with_details(e.to_string()))?;
    }

    Ok(Json(CreateInvoiceResponse { invoice_id }))
}

pub async fn get_invoice(
    State(state): State<AppState>,
    auth: AuthenticatedBusiness,
    Path(invoice_id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let invoice = sqlx::query!(
        r#"
        SELECT id, customer_id, total_amount_cents, state as "state: InvoiceState"
        FROM invoices
        WHERE id = $1 AND business_id = $2
        "#,
        invoice_id,
        auth.business_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to fetch invoice").with_details(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Invoice not found"))?;

    let items = sqlx::query_as!(
        InvoiceItemResponse,
        r#"
        SELECT description, quantity, unit_amount_cents
        FROM invoice_items
        WHERE invoice_id = $1
        "#,
        invoice_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to fetch invoice items").with_details(e.to_string()))?;

    Ok(Json(InvoiceResponse {
        id: invoice.id,
        customer_id: invoice.customer_id,
        total_amount_cents: invoice.total_amount_cents,
        state: invoice.state,
        items,
    }))
}

pub async fn list_invoices(
    State(state): State<AppState>,
    auth: AuthenticatedBusiness,
    Query(filter): Query<InvoiceFilter>,
) -> Result<Json<Vec<InvoiceSummary>>, AppError> {
    let invoices = sqlx::query_as!(
        InvoiceSummary,
        r#"
        SELECT id, customer_id, total_amount_cents, state as "state: InvoiceState"
        FROM invoices
        WHERE business_id = $1 
          AND ($2::TEXT IS NULL OR state = $2)
        "#,
        auth.business_id,
        filter.status as Option<InvoiceState>
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::database("Failed to fetch invoices").with_details(e.to_string()))?;

    Ok(Json(invoices))
}
