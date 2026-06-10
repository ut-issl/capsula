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

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    pub vault: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// A hook output as sent by the CLI to `POST /api/v1/upload`.
///
/// The uploaded variant does not carry a `hook_index` because the server
/// authoritatively assigns positions from each phase's array on receipt
/// (via `.enumerate()`). See [`HookOutputResponse`] for the outbound
/// counterpart which does carry `hook_index`.
#[derive(Debug, Deserialize)]
pub struct HookOutputUpload {
    #[serde(rename = "__meta")]
    pub meta: HookMetaUpload,
    #[serde(flatten)]
    pub output: JsonValue,
}

#[derive(Debug, Deserialize)]
pub struct HookMetaUpload {
    pub id: String,
    pub config: Option<JsonValue>,
    pub success: bool,
    pub error: Option<String>,
}

/// A hook output as returned by the server in run-detail / search responses.
///
/// Includes `hook_index` — the position of this hook in capsula.toml's
/// `pre_run` / `post_run` array — so clients can distinguish the Nth
/// invocation of the same `hook_id`. See [`HookOutputUpload`] for the
/// inbound counterpart which omits it.
#[derive(Debug, Serialize)]
pub struct HookOutputResponse {
    #[serde(rename = "__meta")]
    pub meta: HookMetaResponse,
    #[serde(flatten)]
    pub output: JsonValue,
}

#[derive(Debug, Serialize)]
pub struct HookMetaResponse {
    pub id: String,
    pub config: Option<JsonValue>,
    pub success: bool,
    pub error: Option<String>,
    /// 0-based position of this hook in capsula.toml's `pre_run` /
    /// `post_run` array, matching the DB `run_outputs.hook_index` column.
    pub hook_index: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RunOutputRow {
    pub phase: String,
    pub hook_index: i32,
    pub hook_id: String,
    pub config: Option<JsonValue>,
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

// =============================================================================
// Search API Types
// =============================================================================

/// Request body for POST /api/v1/runs/search
#[derive(Debug, Deserialize)]
pub struct SearchRunsRequest {
    /// Filter by vault name
    pub vault: Option<String>,
    /// Filter runs from this timestamp (ISO 8601)
    pub from: Option<DateTime<Utc>>,
    /// Filter runs until this timestamp (ISO 8601)
    pub to: Option<DateTime<Utc>>,
    /// Filter by exact exit code
    pub exit_code: Option<i32>,
    /// Filter by success (`exit_code` = 0) or failure (`exit_code` != 0)
    pub success: Option<bool>,
    /// Hook output filters (AND logic)
    #[serde(default)]
    pub hook_filters: Vec<HookFilter>,
    /// Structured parameter match filters (AND logic)
    #[serde(default)]
    pub parameter_matches: Vec<ParameterMatch>,
    /// What to include in response
    #[serde(default)]
    pub include: Vec<IncludeField>,
    /// Sort order
    #[serde(default)]
    pub order: SortOrder,
    /// Maximum number of results (default: 100)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

/// A filter condition on a hook's config or output using `JSONPath`
#[derive(Debug, Deserialize)]
pub struct HookFilter {
    /// The hook ID (e.g., "capture-git-repo", "capture-env")
    pub hook_id: String,
    /// `JSONPath` expression to match against hook's config (optional)
    pub config_filter: Option<String>,
    /// `JSONPath` expression to match against hook's output
    pub output_filter: String,
}

/// Comparison operator for parameter matching
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

/// Structured filter for parameter-capturing hooks. See the `capsula-api-types`
/// documentation for the full semantics. The server validates constraints at
/// query-build time.
#[derive(Debug, Deserialize)]
pub struct ParameterMatch {
    pub phase: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub parameter: Option<String>,
    #[serde(default)]
    pub operator: Option<ComparisonOp>,
    #[serde(default)]
    pub value: Option<JsonValue>,
}

/// Fields that can be included in search response
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncludeField {
    Metadata,
    Files,
    Stdout,
    Stderr,
    Hooks,
}

/// Sort order for search results
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    LatestFirst,
    OldestFirst,
}

/// A single run in search results
#[derive(Debug, Serialize)]
pub struct SearchRunResult {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub vault: String,
    pub command: String,
    pub project_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_run_hooks: Option<Vec<HookOutputResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_run_hooks: Option<Vec<HookOutputResponse>>,
}

/// File information in search results
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    pub url: String,
}

/// Response from POST /api/v1/runs/search
#[derive(Debug, Serialize)]
pub struct SearchRunsResponse {
    pub status: String,
    pub total: i64,
    pub runs: Vec<SearchRunResult>,
}
