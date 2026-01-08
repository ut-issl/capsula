use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
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
        .route("/api/vaults", get(list_vaults))
        .route("/api/vaults/{name}", get(get_vault_info))
        .route("/api/runs", post(create_run).get(list_runs))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/upload", post(upload_files))
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

async fn list_vaults(State(pool): State<PgPool>) -> impl IntoResponse {
    info!("Listing all vaults");

    let result = sqlx::query!(
        r#"
        SELECT vault as name, COUNT(*) as "run_count!"
        FROM runs
        GROUP BY vault
        ORDER BY vault
        "#
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(rows) => {
            let vaults: Vec<models::VaultInfo> = rows
                .into_iter()
                .map(|row| models::VaultInfo {
                    name: row.name,
                    run_count: row.run_count,
                })
                .collect();
            info!("Found {} vaults", vaults.len());
            Json(json!({
                "status": "ok",
                "vaults": vaults
            }))
        }
        Err(e) => {
            error!("Failed to list vaults: {}", e);
            Json(json!({
                "status": "error",
                "error": e.to_string()
            }))
        }
    }
}

async fn get_vault_info(State(pool): State<PgPool>, Path(name): Path<String>) -> impl IntoResponse {
    info!("Getting vault info: {}", name);

    let result = sqlx::query!(
        r#"
        SELECT vault as name, COUNT(*) as "run_count!"
        FROM runs
        WHERE vault = $1
        GROUP BY vault
        "#,
        name
    )
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let vault = models::VaultInfo {
                name: row.name,
                run_count: row.run_count,
            };
            info!("Found vault: {} with {} runs", vault.name, vault.run_count);
            Json(json!({
                "status": "ok",
                "exists": true,
                "vault": vault
            }))
        }
        Ok(None) => {
            info!("Vault not found: {}", name);
            Json(json!({
                "status": "ok",
                "exists": false,
                "vault": null
            }))
        }
        Err(e) => {
            error!("Failed to get vault info: {}", e);
            Json(json!({
                "status": "error",
                "error": e.to_string()
            }))
        }
    }
}

async fn list_runs(
    State(pool): State<PgPool>,
    Query(params): Query<models::ListRunsQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    if let Some(ref vault) = params.vault {
        info!(
            "Listing runs for vault: {} (limit={}, offset={})",
            vault, limit, offset
        );
    } else {
        info!("Listing all runs (limit={}, offset={})", limit, offset);
    }

    let result = if let Some(vault) = params.vault {
        sqlx::query_as!(
            models::Run,
            r#"
            SELECT id, name, timestamp, command, vault, project_root,
                   exit_code, duration_ms, stdout, stderr,
                   created_at, updated_at
            FROM runs
            WHERE vault = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
            vault,
            limit,
            offset
        )
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query_as!(
            models::Run,
            r#"
            SELECT id, name, timestamp, command, vault, project_root,
                   exit_code, duration_ms, stdout, stderr,
                   created_at, updated_at
            FROM runs
            ORDER BY timestamp DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&pool)
        .await
    };

    match result {
        Ok(runs) => {
            info!("Found {} runs", runs.len());
            Json(json!({
                "status": "ok",
                "runs": runs,
                "limit": limit,
                "offset": offset
            }))
        }
        Err(e) => {
            error!("Failed to list runs: {}", e);
            Json(json!({
                "status": "error",
                "error": e.to_string()
            }))
        }
    }
}

async fn create_run(
    State(pool): State<PgPool>,
    Json(request): Json<models::CreateRunRequest>,
) -> impl IntoResponse {
    info!(
        "Received run data: id={}, vault={}",
        request.id, request.vault
    );

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

async fn get_run(State(pool): State<PgPool>, Path(id): Path<String>) -> impl IntoResponse {
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

async fn upload_files(mut multipart: Multipart) -> impl IntoResponse {
    info!("Received file upload request");

    let mut files_processed = 0;
    let mut total_bytes = 0u64;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("unknown").to_string();
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        info!(
            "Processing field: name={}, file_name={}, content_type={}",
            name, file_name, content_type
        );

        match field.bytes().await {
            Ok(data) => {
                let size = data.len();
                total_bytes += size as u64;
                files_processed += 1;
                info!("Successfully read field '{}': {} bytes", name, size);
            }
            Err(e) => {
                error!("Failed to read field '{}': {}", name, e);
                return Json(json!({
                    "status": "error",
                    "error": format!("Failed to read field '{}': {}", name, e)
                }));
            }
        }
    }

    info!(
        "Upload complete: {} files, {} bytes total",
        files_processed, total_bytes
    );

    Json(json!({
        "status": "ok",
        "files_processed": files_processed,
        "total_bytes": total_bytes
    }))
}
