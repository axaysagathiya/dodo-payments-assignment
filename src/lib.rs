pub mod auth;
pub mod db;
pub mod error;
pub mod handlers;
pub mod types;

use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn create_app(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/business", post(handlers::create_business))
        .route("/invoices/{id}/pay", post(handlers::pay_invoice));

    let protected = Router::new()
        .route("/customer", post(handlers::create_customer))
        .route("/customers", get(handlers::fetch_all_customers))
        .route("/customers/{id}", get(handlers::get_customer))
        .route("/invoice", post(handlers::create_invoice))
        .route("/invoices", get(handlers::list_invoices))
        .route("/invoice/{id}", get(handlers::get_invoice));

    Router::new()
        .merge(public_routes)
        .merge(protected)
        .with_state(state)
}
