//! Shared API type definitions for Capsula client and server
//!
//! This crate defines the common types used in the Capsula HTTP API to ensure
//! compile-time compatibility between the client and server implementations.

use serde::{Deserialize, Serialize};

/// Information about a vault
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultInfo {
    pub name: String,
    pub run_count: i64,
}

/// Response from the vaults list endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultsResponse {
    pub status: String,
    pub vaults: Vec<VaultInfo>,
}

/// Response from the vault exists endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultExistsResponse {
    pub status: String,
    pub exists: bool,
    pub vault: Option<VaultInfo>,
}

/// Response from the upload run endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadResponse {
    pub status: String,
    pub files_processed: u64,
    pub total_bytes: u64,
    pub pre_run_hooks: u64,
    pub post_run_hooks: u64,
}

/// Generic error response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub status: String,
    pub error: String,
}

// =============================================================================
// Search API Types
// =============================================================================

/// A filter condition on a hook's config or output using `JSONPath`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookFilter {
    /// The hook ID (e.g., "capture-git-repo", "capture-env")
    pub hook_id: String,
    /// `JSONPath` expression to match against hook's config (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_filter: Option<String>,
    /// `JSONPath` expression to match against hook's output
    pub output_filter: String,
}

/// Comparison operator for parameter matching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    /// Exact equality
    Eq,
    /// Not equal
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
}

/// A structured filter for parameter-capturing hooks (e.g., `capture-json`,
/// `capture-toml`, ...).
///
/// Targets `run_outputs` rows produced by hooks whose `output` has a
/// top-level `content` field:
///
/// ```text
/// output: { "content": { ...parsed file contents... } }
/// config: { "path": "<configured-path>", ... }
/// ```
///
/// The server selects matching rows structurally (by the presence of
/// the `content` top-level field) — decoupled from the concrete
/// `hook_id`, so any future hook that emits this shape works
/// automatically.
///
/// Constraints (validated server-side):
///
/// - At least one of `file` / `hook_index` / `parameter` must be specified.
/// - If `parameter` is present, `operator` and `value` must also be present.
/// - If `parameter` is absent, `operator` and `value` must also be absent.
/// - Specifying `parameter` without `file` (and without `hook_index`)
///   emits a server-side warning, since the match will scan across
///   every parameter-capturing row of the run regardless of which
///   file it came from.
///
/// # Example
///
/// ```json
/// {
///     "phase": "pre",
///     "file": "config/sat1/orbit.json",
///     "parameter": "a",
///     "operator": "ge",
///     "value": 1.0
/// }
/// ```
///
/// generates `$.content.a ? (@ >= 1.0)` against the row whose
/// `config.path == "config/sat1/orbit.json"`.
///
/// # Example — using `hook_index`
///
/// Pin the match to the hook at position 1 (0-indexed) in the phase's
/// array, useful when several entries share the same file / `hook_id`:
///
/// ```json
/// {
///     "phase": "pre",
///     "hook_index": 1,
///     "parameter": "a",
///     "operator": "ge",
///     "value": 1.0
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParameterMatch {
    /// Phase: `"pre"` or `"post"`
    pub phase: String,
    /// Optional exact match on the captured file path (compared against
    /// `run_outputs.config->>'path'`). Omitting this causes the match to
    /// apply across every parameter-capturing row of the run; the server
    /// logs a warning when neither `file` nor `hook_index` is supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Optional exact match on the 0-based position of the hook in
    /// `capsula.toml`'s `pre_run` / `post_run` list. Useful when several
    /// entries share the same file / `hook_id` and disambiguation by
    /// position is needed (e.g., "the 2nd `capture-json`").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_index: Option<i32>,
    /// Dot path within the captured `content` object (e.g., `"lr"`,
    /// `"sat1.orbit.a"`). When present, `operator` and `value` are required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
    /// Comparison operator (required iff `parameter` is present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<ComparisonOp>,
    /// Value to compare against — number, string, or boolean. Required
    /// iff `parameter` is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// Fields that can be included in search response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncludeField {
    Metadata,
    Files,
    Stdout,
    Stderr,
    Hooks,
}

/// Sort order for search results
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    LatestFirst,
    OldestFirst,
}

/// Request body for POST /api/v1/runs/search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRunsRequest {
    /// Filter by vault name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,
    /// Filter runs from this timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Filter runs until this timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Filter by exact exit code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Filter by success (`exit_code` = 0) or failure (`exit_code` != 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Hook output filters (AND logic)
    #[serde(default)]
    pub hook_filters: Vec<HookFilter>,
    /// Structured parameter match filters for parameter-capturing hooks
    /// (`capture-json`, `capture-toml`, ...). Each entry is an independent
    /// EXISTS subquery; multiple entries are AND-combined.
    #[serde(default)]
    pub parameter_matches: Vec<ParameterMatch>,
    /// What to include in response
    #[serde(default)]
    pub include: Vec<IncludeField>,
    /// Sort order
    #[serde(default)]
    pub order: SortOrder,
    /// Maximum number of results (default: 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// File information in search results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileInfo {
    pub path: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    pub url: String,
}
