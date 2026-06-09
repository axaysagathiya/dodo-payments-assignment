mod auth;
mod db;
mod error;
mod handlers;
mod types;

use axum::routing::{get, post};
use dotenvy::dotenv;
use sqlx::PgPool;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file, ignore if not present
    let _ = dotenv();

    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("Starting Invoice Payment System...");

    // Initialize database connection pool
    let pool = db::init_pool()
        .await
        .expect("Failed to initialize database pool");

    let state = AppState { db: pool };

    let public_routes = axum::Router::new()
        .route("/business", post(handlers::create_business))
        .route("/invoices/{id}/pay", post(handlers::pay_invoice));

    let protected = axum::Router::new()
        .route("/customer", post(handlers::create_customer))
        .route("/customers", get(handlers::fetch_all_customers))
        .route("/customers/{id}", get(handlers::get_customer))
        .route("/invoice", post(handlers::create_invoice))
        .route("/invoices", get(handlers::list_invoices))
        .route("/invoice/{id}", get(handlers::get_invoice));

    let app = axum::Router::new()
        .merge(public_routes)
        .merge(protected)
        .with_state(state);

    let address = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("Failed to start server");

    info!("Server is listening on {}", address);
    axum::serve(listener, app)
        .await
        .expect("Failed to run server");
}
