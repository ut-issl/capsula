use askama::Template;
use askama_web::WebTemplate;

#[expect(
    clippy::unnecessary_wraps,
    reason = "Askama filter_fn macro generates code that triggers this lint, but Result return type is required by the Askama filter API"
)]
#[expect(
    clippy::inline_always,
    clippy::unused_self,
    reason = "Askama's #[filter_fn] macro generates builder pattern code with #[inline(always)] that triggers these lints"
)]
mod filters {
    use askama::filter_fn;

    #[filter_fn]
    pub fn format_command(s: &str, _env: &dyn askama::Values) -> ::askama::Result<String> {
        // Try to parse as JSON array
        Ok(serde_json::from_str::<Vec<String>>(s).map_or_else(
            |_| s.to_string(),
            |cmd_array| {
                shlex::try_join(cmd_array.iter().map(String::as_str))
                    .unwrap_or_else(|_| cmd_array.join(" "))
            },
        ))
    }

    #[filter_fn]
    pub fn pretty_json<T: std::fmt::Display>(
        s: T,
        _env: &dyn askama::Values,
    ) -> ::askama::Result<String> {
        let s_str = s.to_string();
        // Try to parse as JSON and pretty-print with 2-space indentation
        Ok(serde_json::from_str::<serde_json::Value>(&s_str)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok())
            .unwrap_or(s_str))
    }
}

use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use capsula_api_types::VaultInfo;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::VecDeque;
use std::path::PathBuf;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub storage_path: PathBuf,
}

mod models;
mod query;

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "vaults.html")]
struct VaultsTemplate {
    vaults: Vec<VaultInfo>,
}

#[derive(Template, WebTemplate)]
#[template(path = "runs.html")]
struct RunsTemplate {
    runs: Vec<models::Run>,
    vault: Option<String>,
    page: i64,
    total_pages: i64,
}

#[derive(Template, WebTemplate)]
#[template(path = "run_detail.html")]
struct RunDetailTemplate {
    run: models::Run,
    pre_run_hooks: Vec<models::HookOutput>,
    post_run_hooks: Vec<models::HookOutput>,
    files: Vec<models::CapturedFile>,
}

#[derive(Template, WebTemplate)]
#[template(path = "error.html")]
struct ErrorTemplate {
    status_code: u16,
    title: String,
    message: String,
}

pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(database_url)
        .await
}

pub fn build_app(pool: PgPool, storage_path: PathBuf, max_body_size: usize) -> Router {
    let static_dir: PathBuf = std::env::var("CAPSULA_STATIC_DIR")
        .expect("CAPSULA_STATIC_DIR environment variable must be set")
        .into();

    let state = AppState { pool, storage_path };

    Router::new()
        .route("/", get(index))
        .route("/vaults", get(vaults_page))
        .route("/runs", get(runs_page))
        .route("/runs/{id}", get(run_detail_page))
        .route("/health", get(health_check))
        .route("/api/v1/vaults", get(list_vaults))
        .route("/api/v1/vaults/{name}", get(get_vault_info))
        .route("/api/v1/runs", post(create_run).get(list_runs))
        .route("/api/v1/runs/search", post(search_runs))
        .route("/api/v1/runs/{id}", get(get_run))
        .route("/api/v1/runs/{id}/files/{*path}", get(download_file))
        .route(
            "/api/v1/upload",
            post(upload_files).layer(DefaultBodyLimit::max(max_body_size)),
        )
        .nest_service("/static", ServeDir::new(static_dir))
        .fallback(not_found)
        .with_state(state)
}

async fn index() -> impl IntoResponse {
    IndexTemplate
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        ErrorTemplate {
            status_code: 404,
            title: "Page Not Found".to_string(),
            message: "The page you are looking for does not exist.".to_string(),
        },
    )
}

async fn vaults_page(State(state): State<AppState>) -> impl IntoResponse {
    info!("Rendering vaults page");

    let result = sqlx::query!(
        r#"
        SELECT vault as name, COUNT(*) as "run_count!"
        FROM runs
        GROUP BY vault
        ORDER BY vault
        "#
    )
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(rows) => {
            let vaults: Vec<VaultInfo> = rows
                .into_iter()
                .map(|row| VaultInfo {
                    name: row.name,
                    run_count: row.run_count,
                })
                .collect();
            VaultsTemplate { vaults }
        }
        Err(e) => {
            error!("Failed to fetch vaults: {}", e);
            // Return empty vaults on error
            VaultsTemplate { vaults: Vec::new() }
        }
    }
}

async fn runs_page(
    State(state): State<AppState>,
    Query(params): Query<models::ListRunsQuery>,
) -> impl IntoResponse {
    let page = params.offset.unwrap_or(0) / params.limit.unwrap_or(50) + 1;
    let limit = params.limit.unwrap_or(50);
    let offset = (page - 1) * limit;

    info!(
        "Rendering runs page: page={}, vault={:?}",
        page, params.vault
    );

    // Get total count for pagination
    let total_count = if let Some(ref vault) = params.vault {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)::bigint
            FROM runs
            WHERE vault = $1
            "#,
            vault
        )
        .fetch_one(&state.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0)
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)::bigint
            FROM runs
            "#
        )
        .fetch_one(&state.pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0)
    };

    let total_pages = (total_count + limit - 1) / limit;

    // Fetch runs
    let runs_result = if let Some(ref vault) = params.vault {
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
        .fetch_all(&state.pool)
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
        .fetch_all(&state.pool)
        .await
    };

    match runs_result {
        Ok(runs) => RunsTemplate {
            runs,
            vault: params.vault,
            page,
            total_pages,
        },
        Err(e) => {
            error!("Failed to fetch runs: {}", e);
            RunsTemplate {
                runs: Vec::new(),
                vault: params.vault,
                page: 1,
                total_pages: 1,
            }
        }
    }
}

async fn run_detail_page(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<RunDetailTemplate, StatusCode> {
    info!("Rendering run detail page for: {}", id);

    // Fetch run metadata
    let run = sqlx::query_as!(
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
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        error!("Database error while fetching run: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or_else(|| {
        info!("Run not found: {}", id);
        StatusCode::NOT_FOUND
    })?;

    // Fetch hook outputs
    let hook_outputs_result = sqlx::query_as!(
        models::RunOutputRow,
        r#"
        SELECT phase, hook_id, config, output, success, error
        FROM run_outputs
        WHERE run_id = $1
        ORDER BY id
        "#,
        id
    )
    .fetch_all(&state.pool)
    .await;

    let (pre_run_hooks, post_run_hooks) = match hook_outputs_result {
        Ok(rows) => {
            let mut pre_hooks = Vec::new();
            let mut post_hooks = Vec::new();

            for row in rows {
                let hook_output = models::HookOutput {
                    meta: models::HookMeta {
                        id: row.hook_id,
                        config: row.config,
                        success: row.success,
                        error: row.error,
                    },
                    output: row.output,
                };

                if row.phase == "pre" {
                    pre_hooks.push(hook_output);
                } else if row.phase == "post" {
                    post_hooks.push(hook_output);
                } else {
                    warn!("Unknown hook phase: {}", row.phase);
                }
            }

            (pre_hooks, post_hooks)
        }
        Err(e) => {
            error!("Failed to fetch hook outputs: {}", e);
            (Vec::new(), Vec::new())
        }
    };

    // Fetch captured files
    let files_result = sqlx::query_as!(
        models::CapturedFile,
        r#"
        SELECT path, size, hash, storage_path, content_type
        FROM captured_files
        WHERE run_id = $1
        ORDER BY path
        "#,
        id
    )
    .fetch_all(&state.pool)
    .await;

    let files = match files_result {
        Ok(files) => files,
        Err(e) => {
            error!("Failed to fetch captured files: {}", e);
            Vec::new()
        }
    };

    Ok(RunDetailTemplate {
        run,
        pre_run_hooks,
        post_run_hooks,
        files,
    })
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
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

async fn list_vaults(State(state): State<AppState>) -> impl IntoResponse {
    info!("Listing all vaults");

    let result = sqlx::query!(
        r#"
        SELECT vault as name, COUNT(*) as "run_count!"
        FROM runs
        GROUP BY vault
        ORDER BY vault
        "#
    )
    .fetch_all(&state.pool)
    .await;

    match result {
        Ok(rows) => {
            let vaults: Vec<VaultInfo> = rows
                .into_iter()
                .map(|row| VaultInfo {
                    name: row.name,
                    run_count: row.run_count,
                })
                .collect();
            info!("Found {} vaults", vaults.len());
            let response = capsula_api_types::VaultsResponse {
                status: "ok".to_string(),
                vaults,
            };
            Json(serde_json::to_value(response).expect("Failed to serialize VaultsResponse"))
        }
        Err(e) => {
            error!("Failed to list vaults: {}", e);
            let response = capsula_api_types::ErrorResponse {
                status: "error".to_string(),
                error: e.to_string(),
            };
            Json(serde_json::to_value(response).expect("Failed to serialize ErrorResponse"))
        }
    }
}

async fn get_vault_info(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let vault = VaultInfo {
                name: row.name,
                run_count: row.run_count,
            };
            info!("Found vault: {} with {} runs", vault.name, vault.run_count);
            let response = capsula_api_types::VaultExistsResponse {
                status: "ok".to_string(),
                exists: true,
                vault: Some(vault),
            };
            Json(serde_json::to_value(response).expect("Failed to serialize VaultExistsResponse"))
        }
        Ok(None) => {
            info!("Vault not found: {}", name);
            let response = capsula_api_types::VaultExistsResponse {
                status: "ok".to_string(),
                exists: false,
                vault: None,
            };
            Json(serde_json::to_value(response).expect("Failed to serialize VaultExistsResponse"))
        }
        Err(e) => {
            error!("Failed to get vault info: {}", e);
            let response = capsula_api_types::ErrorResponse {
                status: "error".to_string(),
                error: e.to_string(),
            };
            Json(serde_json::to_value(response).expect("Failed to serialize ErrorResponse"))
        }
    }
}

async fn list_runs(
    State(state): State<AppState>,
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
        .fetch_all(&state.pool)
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
        .fetch_all(&state.pool)
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

/// Search runs with `JSONPath` filters on hook outputs
#[expect(clippy::too_many_lines, reason = "TODO: Refactor later")]
async fn search_runs(
    State(state): State<AppState>,
    Json(request): Json<models::SearchRunsRequest>,
) -> impl IntoResponse {
    info!(
        "Searching runs: vault={:?}, hook_filters={}",
        request.vault,
        request.hook_filters.len()
    );

    // Build the query
    let builder = match query::RunQueryBuilder::from_request(&request) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to build query: {}", e);
            return Json(json!({
                "status": "error",
                "error": e.to_string()
            }));
        }
    };

    let query_sql = builder.build_query();
    let count_sql = builder.build_count_query();
    let bind_values = builder.bind_values();

    info!("Executing search query: {}", query_sql);

    // Execute count query first
    let total: i64 = {
        let mut query = sqlx::query_scalar::<_, i64>(&count_sql);
        for value in bind_values {
            query = match value {
                query::BindValue::String(s) => query.bind(s.clone()),
                query::BindValue::I32(i) => query.bind(*i),
                query::BindValue::I64(i) => query.bind(*i),
                query::BindValue::DateTime(dt) => query.bind(*dt),
                query::BindValue::Bool(b) => query.bind(*b),
            };
        }
        match query.fetch_one(&state.pool).await {
            Ok(count) => count,
            Err(e) => {
                error!("Failed to execute count query: {}", e);
                return Json(json!({
                    "status": "error",
                    "error": "Query execution failed"
                }));
            }
        }
    };

    // Execute main query
    let runs: Vec<models::Run> = {
        let mut query = sqlx::query_as::<_, models::Run>(&query_sql);
        for value in bind_values {
            query = match value {
                query::BindValue::String(s) => query.bind(s.clone()),
                query::BindValue::I32(i) => query.bind(*i),
                query::BindValue::I64(i) => query.bind(*i),
                query::BindValue::DateTime(dt) => query.bind(*dt),
                query::BindValue::Bool(b) => query.bind(*b),
            };
        }
        match query.fetch_all(&state.pool).await {
            Ok(runs) => runs,
            Err(e) => {
                error!("Failed to execute search query: {}", e);
                return Json(json!({
                    "status": "error",
                    "error": "Query execution failed"
                }));
            }
        }
    };

    info!("Found {} runs (total: {})", runs.len(), total);

    // Determine what to include in response
    let include_files = request.include.contains(&models::IncludeField::Files);
    let include_stdout = request.include.contains(&models::IncludeField::Stdout);
    let include_stderr = request.include.contains(&models::IncludeField::Stderr);
    let include_hooks = request.include.contains(&models::IncludeField::Hooks);

    // Build response
    let mut results = Vec::with_capacity(runs.len());
    for run in runs {
        let files = if include_files {
            match sqlx::query_as::<_, models::CapturedFile>(
                "SELECT path, size, hash, storage_path, content_type \
                 FROM captured_files WHERE run_id = $1 ORDER BY path",
            )
            .bind(&run.id)
            .fetch_all(&state.pool)
            .await
            {
                Ok(files) => Some(
                    files
                        .into_iter()
                        .map(|f| models::FileInfo {
                            path: f.path.clone(),
                            size: f.size,
                            hash: f.hash,
                            url: format!("/api/v1/runs/{}/files/{}", run.id, f.path),
                        })
                        .collect(),
                ),
                Err(e) => {
                    warn!("Failed to fetch files for run {}: {}", run.id, e);
                    None
                }
            }
        } else {
            None
        };

        let (pre_run_hooks, post_run_hooks) = if include_hooks {
            match sqlx::query_as::<_, models::RunOutputRow>(
                "SELECT phase, hook_id, config, output, success, error \
                 FROM run_outputs WHERE run_id = $1 ORDER BY id",
            )
            .bind(&run.id)
            .fetch_all(&state.pool)
            .await
            {
                Ok(rows) => {
                    let mut pre_hooks = Vec::new();
                    let mut post_hooks = Vec::new();
                    for row in rows {
                        let hook_output = models::HookOutput {
                            meta: models::HookMeta {
                                id: row.hook_id,
                                config: row.config,
                                success: row.success,
                                error: row.error,
                            },
                            output: row.output,
                        };
                        if row.phase == "pre" {
                            pre_hooks.push(hook_output);
                        } else if row.phase == "post" {
                            post_hooks.push(hook_output);
                        }
                    }
                    (Some(pre_hooks), Some(post_hooks))
                }
                Err(e) => {
                    warn!("Failed to fetch hooks for run {}: {}", run.id, e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        results.push(models::SearchRunResult {
            id: run.id,
            name: run.name,
            timestamp: run.timestamp,
            vault: run.vault,
            command: run.command,
            project_root: run.project_root,
            exit_code: run.exit_code,
            duration_ms: run.duration_ms,
            stdout: if include_stdout { run.stdout } else { None },
            stderr: if include_stderr { run.stderr } else { None },
            files,
            pre_run_hooks,
            post_run_hooks,
        });
    }

    let response = models::SearchRunsResponse {
        status: "ok".to_string(),
        total,
        runs: results,
    };

    Json(serde_json::to_value(response).expect("Failed to serialize SearchRunsResponse"))
}

async fn create_run(
    State(state): State<AppState>,
    Json(request): Json<models::CreateRunRequest>,
) -> impl IntoResponse {
    info!("Creating run: {}", request.id);

    let result = sqlx::query!(
        r#"
        INSERT INTO runs (
            id, name, timestamp, command, vault, project_root,
            exit_code, duration_ms, stdout, stderr
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10
        )
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
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            info!("Run created successfully");
            (
                StatusCode::CREATED,
                Json(json!({
                    "status": "created",
                    "run": request
                })),
            )
        }
        Err(e) => {
            error!("Failed to insert run: {}", e);
            let status = if e.to_string().contains("duplicate key") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "status": "error",
                    "error": e.to_string()
                })),
            )
        }
    }
}

async fn get_run(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    info!("Getting run: {}", id);

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
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(run)) => {
            info!("Found run: {}", run.id);

            // Fetch hook outputs
            let hook_outputs_result = sqlx::query_as!(
                models::RunOutputRow,
                r#"
                SELECT phase, hook_id, config, output, success, error
                FROM run_outputs
                WHERE run_id = $1
                ORDER BY id
                "#,
                id
            )
            .fetch_all(&state.pool)
            .await;

            let (pre_run_hooks, post_run_hooks) = match hook_outputs_result {
                Ok(rows) => {
                    let mut pre_hooks = Vec::new();
                    let mut post_hooks = Vec::new();

                    for row in rows {
                        let hook_output = models::HookOutput {
                            meta: models::HookMeta {
                                id: row.hook_id,
                                config: row.config,
                                success: row.success,
                                error: row.error,
                            },
                            output: row.output,
                        };

                        if row.phase == "pre" {
                            pre_hooks.push(hook_output);
                        } else if row.phase == "post" {
                            post_hooks.push(hook_output);
                        } else {
                            warn!("Unknown hook phase: {}", row.phase);
                        }
                    }

                    info!(
                        "Found {} pre-run hooks and {} post-run hooks",
                        pre_hooks.len(),
                        post_hooks.len()
                    );
                    (pre_hooks, post_hooks)
                }
                Err(e) => {
                    error!("Failed to fetch hook outputs: {}", e);
                    (Vec::new(), Vec::new())
                }
            };

            Json(json!({
                "status": "ok",
                "run": run,
                "pre_run_hooks": pre_run_hooks,
                "post_run_hooks": post_run_hooks
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

#[expect(clippy::too_many_lines, reason = "TODO: Refactor later")]
#[expect(
    clippy::else_if_without_else,
    reason = "There is `continue` or `return` in each branch, so `else` is redundant"
)]
async fn upload_files(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let storage_path = &state.storage_path;
    info!("Received file upload request");

    if let Err(e) = tokio::fs::create_dir_all(&storage_path).await {
        error!(
            "Failed to create storage directory at {}: {}",
            storage_path.display(),
            e
        );
        return Json(json!({
            "status": "error",
            "error": format!("Failed to create storage directory: {}", e)
        }));
    }

    let mut files_processed = 0;
    let mut total_bytes = 0u64;
    let mut run_id: Option<String> = None;
    let mut pending_paths: VecDeque<String> = VecDeque::new();
    let mut pre_run_hooks: Option<Vec<models::HookOutput>> = None;
    let mut post_run_hooks: Option<Vec<models::HookOutput>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("unknown").to_string();

        if field_name == "run_id" {
            match field.text().await {
                Ok(text) => {
                    run_id = Some(text);
                    continue;
                }
                Err(e) => {
                    error!("Failed to read run_id field: {}", e);
                    return Json(json!({
                        "status": "error",
                        "error": format!("Failed to read run_id: {}", e)
                    }));
                }
            }
        } else if field_name == "path" {
            match field.text().await {
                Ok(text) => {
                    pending_paths.push_back(text);
                    continue;
                }
                Err(e) => {
                    error!("Failed to read path field: {}", e);
                    return Json(json!({
                        "status": "error",
                        "error": format!("Failed to read path: {}", e)
                    }));
                }
            }
        } else if field_name == "pre_run" {
            match field.text().await {
                Ok(text) => match serde_json::from_str::<Vec<models::HookOutput>>(&text) {
                    Ok(hooks) => {
                        info!("Parsed {} pre-run hooks", hooks.len());
                        pre_run_hooks = Some(hooks);
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to parse pre_run JSON: {}", e);
                        return Json(json!({
                            "status": "error",
                            "error": format!("Failed to parse pre_run JSON: {}", e)
                        }));
                    }
                },
                Err(e) => {
                    error!("Failed to read pre_run field: {}", e);
                    return Json(json!({
                        "status": "error",
                        "error": format!("Failed to read pre_run: {}", e)
                    }));
                }
            }
        } else if field_name == "post_run" {
            match field.text().await {
                Ok(text) => match serde_json::from_str::<Vec<models::HookOutput>>(&text) {
                    Ok(hooks) => {
                        info!("Parsed {} post-run hooks", hooks.len());
                        post_run_hooks = Some(hooks);
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to parse post_run JSON: {}", e);
                        return Json(json!({
                            "status": "error",
                            "error": format!("Failed to parse post_run JSON: {}", e)
                        }));
                    }
                },
                Err(e) => {
                    error!("Failed to read post_run field: {}", e);
                    return Json(json!({
                        "status": "error",
                        "error": format!("Failed to read post_run: {}", e)
                    }));
                }
            }
        }

        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        info!(
            "Processing file: field_name={}, file_name={}, content_type={}",
            field_name, file_name, content_type
        );

        match field.bytes().await {
            Ok(data) => {
                let size = data.len();
                total_bytes += size as u64;
                let Ok(size_i64) = i64::try_from(size) else {
                    error!("File too large to store size in database: {} bytes", size);
                    return Json(json!({
                        "status": "error",
                        "error": "File too large to store size in database"
                    }));
                };

                let mut hasher = Sha256::new();
                hasher.update(&data);
                let hash = format!("{:x}", hasher.finalize());

                info!("File hash: {}, size: {} bytes", hash, size);

                let hash_dir = &hash[0..2];
                let file_storage_dir = storage_path.join(hash_dir);
                if let Err(e) = tokio::fs::create_dir_all(&file_storage_dir).await {
                    error!(
                        "Failed to create hash directory at {}: {}",
                        file_storage_dir.display(),
                        e
                    );
                    return Json(json!({
                        "status": "error",
                        "error": format!("Failed to create storage directory: {}", e)
                    }));
                }

                let file_storage_path = file_storage_dir.join(&hash);
                let storage_path_str = file_storage_path.to_string_lossy().to_string();

                if file_storage_path.exists() {
                    info!("File already exists (deduplicated): {}", storage_path_str);
                } else {
                    if let Err(e) = tokio::fs::write(&file_storage_path, &data).await {
                        error!("Failed to write file: {}", e);
                        return Json(json!({
                            "status": "error",
                            "error": format!("Failed to write file: {}", e)
                        }));
                    }
                    info!("Saved new file: {}", storage_path_str);
                }

                if let Some(ref rid) = run_id {
                    let relative_path = pending_paths
                        .pop_front()
                        .or_else(|| {
                            if file_name == "unknown" {
                                None
                            } else {
                                Some(file_name.clone())
                            }
                        })
                        .unwrap_or_else(|| {
                            if field_name != "unknown" && field_name != "file" {
                                field_name.clone()
                            } else {
                                format!("file-{}", files_processed + 1)
                            }
                        });

                    let result = sqlx::query!(
                        r#"
                        INSERT INTO captured_files (run_id, path, size, hash, storage_path, content_type)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        ON CONFLICT (run_id, path) DO UPDATE
                        SET size = EXCLUDED.size,
                            hash = EXCLUDED.hash,
                            storage_path = EXCLUDED.storage_path,
                            content_type = EXCLUDED.content_type
                        "#,
                        rid,
                        relative_path,
                        size_i64,
                        hash,
                        storage_path_str,
                        content_type
                    )
                    .execute(&state.pool)
                    .await;

                    match result {
                        Ok(_) => {
                            info!("Stored file metadata in database");
                        }
                        Err(e) => {
                            error!("Failed to store file metadata: {}", e);
                            return Json(json!({
                                "status": "error",
                                "error": format!("Failed to store file metadata: {}", e)
                            }));
                        }
                    }
                }

                files_processed += 1;
                info!("Successfully processed file: {} bytes", size);
            }
            Err(e) => {
                error!("Failed to read file field: {}", e);
                return Json(json!({
                    "status": "error",
                    "error": format!("Failed to read file: {}", e)
                }));
            }
        }
    }

    info!(
        "Upload complete: {} files, {} bytes total",
        files_processed, total_bytes
    );

    // Store hook outputs if provided
    let mut pre_run_count = 0;
    let mut post_run_count = 0;

    if let Some(ref rid) = run_id {
        if let Some(hooks) = pre_run_hooks {
            for hook in hooks {
                let result = sqlx::query!(
                    r#"
                    INSERT INTO run_outputs (run_id, phase, hook_id, config, output, success, error)
                    VALUES ($1, 'pre', $2, $3, $4, $5, $6)
                    "#,
                    rid,
                    hook.meta.id,
                    hook.meta.config,
                    hook.output,
                    hook.meta.success,
                    hook.meta.error
                )
                .execute(&state.pool)
                .await;

                match result {
                    Ok(_) => {
                        pre_run_count += 1;
                        info!("Stored pre-run hook: {}", hook.meta.id);
                    }
                    Err(e) => {
                        error!("Failed to store pre-run hook {}: {}", hook.meta.id, e);
                        return Json(json!({
                            "status": "error",
                            "error": format!("Failed to store pre-run hook {}: {}", hook.meta.id, e)
                        }));
                    }
                }
            }
        }

        if let Some(hooks) = post_run_hooks {
            for hook in hooks {
                let result = sqlx::query!(
                    r#"
                    INSERT INTO run_outputs (run_id, phase, hook_id, config, output, success, error)
                    VALUES ($1, 'post', $2, $3, $4, $5, $6)
                    "#,
                    rid,
                    hook.meta.id,
                    hook.meta.config,
                    hook.output,
                    hook.meta.success,
                    hook.meta.error
                )
                .execute(&state.pool)
                .await;

                match result {
                    Ok(_) => {
                        post_run_count += 1;
                        info!("Stored post-run hook: {}", hook.meta.id);
                    }
                    Err(e) => {
                        error!("Failed to store post-run hook {}: {}", hook.meta.id, e);
                        return Json(json!({
                            "status": "error",
                            "error": format!("Failed to store post-run hook {}: {}", hook.meta.id, e)
                        }));
                    }
                }
            }
        }
    }

    info!(
        "Hook outputs stored: {} pre-run, {} post-run",
        pre_run_count, post_run_count
    );

    let response = capsula_api_types::UploadResponse {
        status: "ok".to_string(),
        files_processed,
        total_bytes,
        pre_run_hooks: pre_run_count,
        post_run_hooks: post_run_count,
    };
    Json(serde_json::to_value(response).expect("Failed to serialize UploadResponse"))
}

async fn download_file(
    State(state): State<AppState>,
    Path((run_id, file_path)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    info!("Downloading file: run_id={}, path={}", run_id, file_path);

    // Query the database for file metadata
    let file_record = sqlx::query!(
        r#"
        SELECT storage_path, content_type, path as file_path
        FROM captured_files
        WHERE run_id = $1 AND path = $2
        "#,
        run_id,
        file_path
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        error!("Database error while fetching file metadata: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(record) = file_record else {
        info!("File not found: run_id={}, path={}", run_id, file_path);
        return Err(StatusCode::NOT_FOUND);
    };

    // Read the file from storage
    let file_data = tokio::fs::read(&record.storage_path).await.map_err(|e| {
        error!(
            "Failed to read file from storage {}: {}",
            record.storage_path, e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(
        "Successfully read file: {} bytes from {}",
        file_data.len(),
        record.storage_path
    );

    // Determine content type (use stored value or guess from filename)
    let content_type = record.content_type.unwrap_or_else(|| {
        mime_guess::from_path(&record.file_path)
            .first_or_octet_stream()
            .to_string()
    });

    // Extract filename from path for Content-Disposition
    let filename = std::path::Path::new(&record.file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");

    // Build response with appropriate headers
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{filename}\""),
        )
        .body(Body::from(file_data))
        .map_err(|e| {
            error!("Failed to build response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
