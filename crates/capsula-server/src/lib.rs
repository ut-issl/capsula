//! Capsula server: multi-tenant storage and browsing of Capsula runs.
//!
//! The server is being rewritten around organizations, vaults and runs
//! (API v2, see <https://github.com/ut-issl/capsula/issues/1352>). The
//! single-tenant API v1 and its pages were removed together with the old
//! schema; the v2 endpoints are added resource by resource.

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Router,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json},
    routing::get,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

pub mod config;
pub mod db;
pub mod paths;
pub mod storage;

use config::Config;
use storage::Storage;

/// Everything the handlers need. Cheap to clone; clones share the pool, the
/// storage backend and the configuration.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub storage: Storage,
    pub config: Arc<Config>,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool, storage: Storage, config: Config) -> Self {
        Self {
            pool,
            storage,
            config: Arc::new(config),
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
struct IndexTemplate;

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

/// The embedded database migrations.
#[must_use]
pub fn migrator() -> sqlx::migrate::Migrator {
    sqlx::migrate!("./migrations")
}

/// Compose the router. Routers of the individual resources are nested here.
pub fn build_app(state: AppState) -> Router {
    let max_body_size = state.config.max_body_size;

    Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/static/style.css", get(serve_style_css))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(max_body_size))
        .with_state(state)
}

async fn serve_style_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, mime_guess::mime::TEXT_CSS.as_ref())],
        include_str!("../static/style.css"),
    )
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
