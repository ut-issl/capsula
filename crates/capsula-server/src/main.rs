use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::json;
use sqlx::PgPool;
use tracing::info;

mod models;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");

    info!("Connecting to database: {}", database_url);

    // Create database connection pool with options
    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            info!("Database connection established");
            pool
        }
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // Build router with routes
    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(health_check))
        .route("/api/runs", post(create_run))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to port 3000");

    info!("Server listening on http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Capsula Server</h1><p>Hello World!</p>")
}

async fn health_check(State(pool): State<PgPool>) -> impl IntoResponse {
    // Try to execute a simple query to check database connection
    match sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => Json(json!({
            "status": "ok",
            "database": "connected"
        })),
        Err(e) => Json(json!({
            "status": "error",
            "database": "disconnected",
            "error": e.to_string()
        })),
    }
}

async fn create_run(
    State(_pool): State<PgPool>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("Received run data: {}", payload);
    
    // Echo the received payload back with additional metadata
    Json(json!({
        "status": "received",
        "data": payload
    }))
}
