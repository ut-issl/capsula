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

/// Hook execution phase.
///
/// Serialized as `"pre"` / `"post"` on the wire.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Pre,
    Post,
}

/// A single `<parameter> <operator> <value>` comparison inside the
/// captured `content` object. See [`ParameterMatch::conditions`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParameterCondition {
    pub parameter: String,
    pub operator: ComparisonOp,
    pub value: serde_json::Value,
}

/// A structured filter over parameter-capturing hooks (`capture-json`,
/// `capture-toml`, ...).
///
/// Selects `run_outputs` rows structurally by the presence of a
/// top-level `content` field, then optionally pins by `file` /
/// `hook_index` and/or filters by `conditions`.
///
/// # Why `conditions` is a list, not a single `parameter` + `operator` + `value`
///
/// Two entries at the `parameter_matches` level are AND-combined across
/// *rows* — `lr >= 0.005` in one entry and `lr <= 0.05` in another are
/// satisfied by a run that has *some* row above the lower bound and
/// *some (possibly different)* row below the upper. Bundling them
/// inside one `ParameterMatch` compiles both into a single `JSONPath`
/// predicate applied to the same row, which is what a caller writing a
/// range actually wants.
///
/// # Example — range on the same field
///
/// ```json
/// {
///     "phase": "pre",
///     "file": "train.json",
///     "conditions": [
///         { "parameter": "lr", "operator": "ge", "value": 0.005 },
///         { "parameter": "lr", "operator": "le", "value": 0.05 }
///     ]
/// }
/// ```
///
/// compiles to `$ ? (@.content.lr >= 0.005 && @.content.lr <= 0.05)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParameterMatch {
    pub phase: Phase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// 0-based position of the hook in `capsula.toml`'s `pre_run` /
    /// `post_run` list. `i32` for parity with the DB column; negatives
    /// are rejected server-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_index: Option<i32>,
    /// May be empty when `file` or `hook_index` is set — the match then
    /// only asserts a parameter-capturing row exists.
    #[serde(default)]
    pub conditions: Vec<ParameterCondition>,
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
    /// Independent `ParameterMatch` entries, AND-combined at the run
    /// level. To share a row between two conditions (e.g. a range) put
    /// them inside a single entry's `conditions`, not across entries —
    /// see [`ParameterMatch`]. Capped at 32 entries per request.
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
