use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Run {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub vault: String,
    pub project_root: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRunRequest {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub vault: String,
    pub project_root: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub name: String,
    pub run_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    pub vault: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookOutput {
    #[serde(rename = "__meta")]
    pub meta: HookMeta,
    #[serde(flatten)]
    pub output: JsonValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookMeta {
    pub id: String,
    pub config: Option<JsonValue>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RunOutputRow {
    pub phase: String,
    pub hook_id: String,
    pub output: JsonValue,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CapturedFile {
    pub path: String,
    pub size: i64,
    pub hash: Option<String>,
    pub storage_path: String,
    pub content_type: Option<String>,
}
