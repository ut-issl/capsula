use axum::{
    Router,
    extract::{Path, State},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
};
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info};

mod models;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

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
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // Build router with routes
    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(health_check))
        .route("/api/runs", post(create_run))
        .route("/api/runs/{id}", get(get_run))
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
    match sqlx::query("SELECT 1").fetch_one(&pool).await {
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
    State(pool): State<PgPool>,
    Json(request): Json<models::CreateRunRequest>,
) -> impl IntoResponse {
    info!("Received run data: id={}, vault={}", request.id, request.vault);

    // Insert run into database
    let result = sqlx::query!(
        r#"
        INSERT INTO runs (
            id, name, timestamp, command, vault, project_root,
            exit_code, duration_ms, stdout, stderr
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, name, timestamp, command, vault, project_root,
                  exit_code, duration_ms, stdout, stderr,
                  created_at, updated_at
        "#,
        request.id,
        request.name,
        request.timestamp,
        request.command,
        request.vault,
        request.project_root,
        request.exit_code,
        request.duration_ms,
        request.stdout,
        request.stderr
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(row) => {
            let run = models::Run {
                id: row.id,
                name: row.name,
                timestamp: row.timestamp,
                command: row.command,
                vault: row.vault,
                project_root: row.project_root,
                exit_code: row.exit_code,
                duration_ms: row.duration_ms,
                stdout: row.stdout,
                stderr: row.stderr,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            info!("Successfully created run: {}", run.id);
            Json(json!({
                "status": "created",
                "run": run
            }))
        }
        Err(e) => {
            error!("Failed to insert run: {}", e);
            Json(json!({
                "status": "error",
                "error": e.to_string()
            }))
        }
    }
}

async fn get_run(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Retrieving run: {}", id);

    let result = sqlx::query_as!(
        models::Run,
        r#"
        SELECT id, name, timestamp, command, vault, project_root,
               exit_code, duration_ms, stdout, stderr,
               created_at, updated_at
        FROM runs
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(run)) => {
            info!("Found run: {}", run.id);
            Json(json!({
                "status": "ok",
                "run": run
            }))
        }
        Ok(None) => {
            info!("Run not found: {}", id);
            Json(json!({
                "status": "not_found",
                "error": format!("Run with id {} not found", id)
            }))
        }
        Err(e) => {
            error!("Failed to retrieve run: {}", e);
            Json(json!({
                "status": "error",
                "error": e.to_string()
            }))
        }
    }
}
