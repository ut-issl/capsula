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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// A filter condition on a hook's config or output using JSONPath
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookFilter {
    /// The hook ID (e.g., "capture-git-repo", "capture-env")
    pub hook_id: String,
    /// JSONPath expression to match against hook's config (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_filter: Option<String>,
    /// JSONPath expression to match against hook's output
    pub output_filter: String,
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
    /// Filter by success (exit_code = 0) or failure (exit_code != 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Hook output filters (AND logic)
    #[serde(default)]
    pub hook_filters: Vec<HookFilter>,
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
