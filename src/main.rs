use dotenvy::dotenv;
use invoice_payment_system::{AppState, create_app, db};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

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
    let app = create_app(state);

    let address = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("Failed to start server");

    info!("Server is listening on {}", address);
    axum::serve(listener, app)
        .await
        .expect("Failed to run server");
}
