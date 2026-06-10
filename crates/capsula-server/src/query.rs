//! Query builder for constructing SQL queries with `JSONPath` filters
//!
//! This module provides a builder for constructing dynamic SQL queries
//! that filter runs based on metadata and hook outputs using `JSONPath` expressions.

use crate::models::{ComparisonOp, HookFilter, ParameterMatch, SearchRunsRequest, SortOrder};
use chrono::{DateTime, Utc};
use sql_json_path::JsonPath;
use std::fmt::Write;

/// Maximum length for `JSONPath` expressions (prevents denial-of-service)
const MAX_JSONPATH_LENGTH: usize = 500;

/// Maximum LIMIT value: callers asking for more than this are clamped.
const MAX_LIMIT: i64 = 1_000;

/// Maximum OFFSET value: callers asking for more than this are clamped.
/// Prevents slow-query `DoS` from a large `OFFSET` forcing a sequential scan.
const MAX_OFFSET: i64 = 100_000;

/// Error types for query building
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Invalid JSONPath expression: {0}")]
    InvalidJsonPath(String),
}

/// Builder for constructing run search queries
#[derive(Debug, Default)]
pub struct RunQueryBuilder {
    /// WHERE conditions for the runs table
    conditions: Vec<String>,
    /// EXISTS subqueries for hook filters
    hook_exists_clauses: Vec<String>,
    /// Bind parameters (as strings for now, will be used with sqlx)
    bind_values: Vec<BindValue>,
    /// Current parameter index
    param_index: usize,
    /// Sort order
    order: SortOrder,
    /// Limit
    limit: i64,
    /// Offset
    offset: i64,
}

/// A bind value for SQL queries
#[derive(Debug, Clone)]
#[expect(dead_code, reason = "Some variants are reserved for future use")]
pub enum BindValue {
    String(String),
    I32(i32),
    I64(i64),
    DateTime(DateTime<Utc>),
    Bool(bool),
}

impl RunQueryBuilder {
    /// Create a new query builder
    pub const fn new() -> Self {
        Self {
            conditions: Vec::new(),
            hook_exists_clauses: Vec::new(),
            bind_values: Vec::new(),
            param_index: 1,
            order: SortOrder::LatestFirst,
            limit: 100,
            offset: 0,
        }
    }

    /// Build from a search request
    pub fn from_request(request: &SearchRunsRequest) -> Result<Self, QueryError> {
        let mut builder = Self::new();

        if let Some(vault) = &request.vault {
            builder = builder.with_vault(vault);
        }

        if let Some(from) = request.from {
            builder = builder.with_from_timestamp(from);
        }

        if let Some(to) = request.to {
            builder = builder.with_to_timestamp(to);
        }

        if let Some(exit_code) = request.exit_code {
            builder = builder.with_exit_code(exit_code);
        }

        if let Some(success) = request.success {
            builder = builder.with_success(success);
        }

        for hook_filter in &request.hook_filters {
            builder = builder.with_hook_filter(hook_filter)?;
        }

        for parameter_match in &request.parameter_matches {
            builder = builder.with_parameter_match(parameter_match)?;
        }

        builder.order = request.order.clone();
        builder.limit = request.limit.unwrap_or(100);
        builder.offset = request.offset.unwrap_or(0);

        Ok(builder)
    }

    /// Add a vault filter
    pub fn with_vault(mut self, vault: &str) -> Self {
        self.conditions
            .push(format!("r.vault = ${}", self.param_index));
        self.bind_values.push(BindValue::String(vault.to_string()));
        self.param_index += 1;
        self
    }

    /// Add a from timestamp filter
    pub fn with_from_timestamp(mut self, from: DateTime<Utc>) -> Self {
        self.conditions
            .push(format!("r.timestamp >= ${}", self.param_index));
        self.bind_values.push(BindValue::DateTime(from));
        self.param_index += 1;
        self
    }

    /// Add a to timestamp filter
    pub fn with_to_timestamp(mut self, to: DateTime<Utc>) -> Self {
        self.conditions
            .push(format!("r.timestamp <= ${}", self.param_index));
        self.bind_values.push(BindValue::DateTime(to));
        self.param_index += 1;
        self
    }

    /// Add an exit code filter
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.conditions
            .push(format!("r.exit_code = ${}", self.param_index));
        self.bind_values.push(BindValue::I32(exit_code));
        self.param_index += 1;
        self
    }

    /// Add a success filter (`exit_code` = 0 or `exit_code` != 0)
    pub fn with_success(mut self, success: bool) -> Self {
        if success {
            self.conditions.push("r.exit_code = 0".to_string());
        } else {
            self.conditions
                .push("r.exit_code IS NOT NULL AND r.exit_code != 0".to_string());
        }
        self
    }

    /// Add a hook filter using `JSONPath`
    pub fn with_hook_filter(mut self, filter: &HookFilter) -> Result<Self, QueryError> {
        // Validate JSONPath expressions (basic validation)
        Self::validate_jsonpath(&filter.output_filter)?;
        if let Some(config_filter) = &filter.config_filter {
            Self::validate_jsonpath(config_filter)?;
        }

        // Build the EXISTS subquery
        let mut subquery_conditions = vec![
            "ro.run_id = r.id".to_string(),
            format!("ro.hook_id = ${}", self.param_index),
        ];
        self.bind_values
            .push(BindValue::String(filter.hook_id.clone()));
        self.param_index += 1;

        // Add config filter if present
        if let Some(config_filter) = &filter.config_filter {
            subquery_conditions.push(format!(
                "jsonb_path_exists(ro.config, ${}::jsonpath)",
                self.param_index
            ));
            self.bind_values
                .push(BindValue::String(config_filter.clone()));
            self.param_index += 1;
        }

        // Add output filter
        subquery_conditions.push(format!(
            "jsonb_path_exists(ro.output, ${}::jsonpath)",
            self.param_index
        ));
        self.bind_values
            .push(BindValue::String(filter.output_filter.clone()));
        self.param_index += 1;

        let exists_clause = format!(
            "EXISTS (SELECT 1 FROM run_outputs ro WHERE {})",
            subquery_conditions.join(" AND ")
        );
        self.hook_exists_clauses.push(exists_clause);

        Ok(self)
    }

    /// Add a structured parameter match filter targeting parameter-capturing
    /// hooks (`capture-json`, `capture-toml`, ...).
    ///
    /// Those hooks share an output shape — a top-level `content` field
    /// containing the parsed file — and store the configured path under
    /// `__meta.config.path` (persisted in the `config` column of
    /// `run_outputs`). This method:
    ///
    /// 1. Selects rows structurally with `ro.output ? 'content'`, so the
    ///    filter is decoupled from the concrete `hook_id`.
    /// 2. Optionally pins to a specific captured file by matching
    ///    `ro.config->>'path'` against the supplied `file`.
    /// 3. Optionally adds a `JSONPath` predicate
    ///    `$.content.<parameter> ? (@ <op> <value>)` on `ro.output`.
    ///
    /// Validation:
    /// - `phase` must be `"pre"` or `"post"`.
    /// - At least one of `file` / `parameter` must be specified.
    /// - `parameter`, `operator`, and `value` are an all-or-nothing group.
    /// - Specifying `parameter` without `file` emits a warning.
    pub fn with_parameter_match(mut self, pm: &ParameterMatch) -> Result<Self, QueryError> {
        // Validate phase
        let phase = match pm.phase.as_str() {
            "pre" | "post" => pm.phase.clone(),
            _ => {
                return Err(QueryError::InvalidJsonPath(
                    "phase must be 'pre' or 'post'".to_string(),
                ));
            }
        };

        // Validate the parameter/operator/value triple is all-or-nothing
        let condition = match (&pm.parameter, &pm.operator, &pm.value) {
            (None, None, None) => None,
            (Some(p), Some(op), Some(v)) => Some((p.as_str(), op, v)),
            _ => {
                return Err(QueryError::InvalidJsonPath(
                    "parameter, operator, and value must all be specified together".to_string(),
                ));
            }
        };

        // At least one of file / parameter must be present, otherwise the
        // filter would match every parameter-capturing row of the run.
        if pm.file.is_none() && condition.is_none() {
            return Err(QueryError::InvalidJsonPath(
                "ParameterMatch requires at least one of 'file' or 'parameter'".to_string(),
            ));
        }

        if condition.is_some() && pm.file.is_none() {
            tracing::warn!(
                "ParameterMatch without 'file' will match across every \
                 parameter-capturing row of the run; specify 'file' to narrow"
            );
        }

        // Build EXISTS subquery
        let mut conds = vec![
            "ro.run_id = r.id".to_string(),
            format!("ro.phase = ${}", self.param_index),
        ];
        self.bind_values.push(BindValue::String(phase));
        self.param_index += 1;

        // Structural filter: only parameter-capturing rows have `content`.
        conds.push("ro.output ? 'content'".to_string());

        if let Some(file) = &pm.file {
            conds.push(format!("ro.config->>'path' = ${}", self.param_index));
            self.bind_values.push(BindValue::String(file.clone()));
            self.param_index += 1;
        }

        if let Some((param, op, value)) = condition {
            let jsonpath = build_parameter_jsonpath(param, op, value)?;
            Self::validate_jsonpath(&jsonpath)?;
            conds.push(format!(
                "jsonb_path_exists(ro.output, ${}::jsonpath)",
                self.param_index
            ));
            self.bind_values.push(BindValue::String(jsonpath));
            self.param_index += 1;
        }

        let exists = format!(
            "EXISTS (SELECT 1 FROM run_outputs ro WHERE {})",
            conds.join(" AND ")
        );
        self.hook_exists_clauses.push(exists);

        Ok(self)
    }

    /// Validate a `JSONPath` expression using SQL/JSON path parser
    ///
    /// Uses `sql-json-path` crate which is compatible with `PostgreSQL`'s SQL/JSON
    /// path language, including `starts with`, `like_regex`, `.type()`, `.size()`,
    /// arithmetic operators, etc.
    fn validate_jsonpath(expr: &str) -> Result<(), QueryError> {
        // Length limit for DoS prevention
        if expr.len() > MAX_JSONPATH_LENGTH {
            return Err(QueryError::InvalidJsonPath(
                "JSONPath expression too long (max 500 characters)".to_string(),
            ));
        }

        // Parse using SQL/JSON path parser (PostgreSQL compatible)
        JsonPath::new(expr)
            .map_err(|e| QueryError::InvalidJsonPath(format!("Invalid JSONPath syntax: {e}")))?;

        Ok(())
    }

    /// Build the main SELECT query
    pub fn build_query(&self) -> String {
        let mut query = String::from(
            "SELECT DISTINCT r.id, r.name, r.timestamp, r.command, r.vault, r.project_root, \
             r.exit_code, r.duration_ms, r.stdout, r.stderr, r.created_at, r.updated_at \
             FROM runs r",
        );

        // Add WHERE clause
        let all_conditions: Vec<&str> = self
            .conditions
            .iter()
            .map(String::as_str)
            .chain(self.hook_exists_clauses.iter().map(String::as_str))
            .collect();

        if !all_conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&all_conditions.join(" AND "));
        }

        // Add ORDER BY
        match self.order {
            SortOrder::LatestFirst => query.push_str(" ORDER BY r.timestamp DESC"),
            SortOrder::OldestFirst => query.push_str(" ORDER BY r.timestamp ASC"),
        }

        // Add LIMIT and OFFSET (clamped to avoid DoS from large scans)
        let _ = write!(
            query,
            " LIMIT {} OFFSET {}",
            self.limit.clamp(0, MAX_LIMIT),
            self.offset.clamp(0, MAX_OFFSET),
        );

        query
    }

    /// Build the COUNT query for total results
    pub fn build_count_query(&self) -> String {
        let mut query = String::from("SELECT COUNT(DISTINCT r.id) FROM runs r");

        let all_conditions: Vec<&str> = self
            .conditions
            .iter()
            .map(String::as_str)
            .chain(self.hook_exists_clauses.iter().map(String::as_str))
            .collect();

        if !all_conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&all_conditions.join(" AND "));
        }

        query
    }

    /// Get bind values for the query
    pub fn bind_values(&self) -> &[BindValue] {
        &self.bind_values
    }
}

/// Build a PostgreSQL-compatible `JSONPath` expression for a single
/// parameter comparison inside the captured `content` object.
///
/// The path is rooted at `$.content` (the field produced by parameter-capturing
/// hooks), e.g., `$.content.sat1.orbit.a ? (@ >= 1.0)`.
fn build_parameter_jsonpath(
    parameter: &str,
    operator: &ComparisonOp,
    value: &serde_json::Value,
) -> Result<String, QueryError> {
    // Validate parameter name (1-200 chars of [A-Za-z0-9_.])
    if parameter.is_empty() || parameter.len() > 200 {
        return Err(QueryError::InvalidJsonPath(
            "parameter name must be 1-200 characters".to_string(),
        ));
    }
    for ch in parameter.chars() {
        if !ch.is_alphanumeric() && ch != '_' && ch != '.' {
            return Err(QueryError::InvalidJsonPath(format!(
                "parameter name contains invalid character: '{ch}'"
            )));
        }
    }

    let op_str = match operator {
        ComparisonOp::Eq => "==",
        ComparisonOp::Ne => "!=",
        ComparisonOp::Gt => ">",
        ComparisonOp::Ge => ">=",
        ComparisonOp::Lt => "<",
        ComparisonOp::Le => "<=",
    };

    let value_str = match value {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        serde_json::Value::Bool(b) => b.to_string(),
        _ => {
            return Err(QueryError::InvalidJsonPath(
                "value must be number, string, or boolean".to_string(),
            ));
        }
    };

    Ok(format!("$.content.{parameter} ? (@ {op_str} {value_str})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query() {
        let builder = RunQueryBuilder::new().with_vault("my-project");
        let query = builder.build_query();
        assert!(query.contains("r.vault = $1"));
        assert!(query.contains("ORDER BY r.timestamp DESC"));
    }

    #[test]
    fn test_hook_filter() {
        let filter = HookFilter {
            hook_id: "capture-git-repo".to_string(),
            config_filter: None,
            output_filter: "$.sha ? (@ starts with \"abc\")".to_string(),
        };
        let builder = RunQueryBuilder::new()
            .with_hook_filter(&filter)
            .expect("valid JSONPath filter should succeed");
        let query = builder.build_query();
        assert!(query.contains("EXISTS"));
        assert!(query.contains("jsonb_path_exists"));
    }

    #[test]
    fn test_invalid_jsonpath() {
        let filter = HookFilter {
            hook_id: "test".to_string(),
            config_filter: None,
            output_filter: "not-starting-with-dollar".to_string(),
        };
        let result = RunQueryBuilder::new().with_hook_filter(&filter);
        assert!(result.is_err());
    }

    #[test]
    fn test_combined_filters() {
        let filter = HookFilter {
            hook_id: "capture-env".to_string(),
            config_filter: Some("$.name ? (@ == \"PARAM1\")".to_string()),
            output_filter: "$.value ? (@ == \"production\")".to_string(),
        };
        let builder = RunQueryBuilder::new()
            .with_vault("my-project")
            .with_success(true)
            .with_hook_filter(&filter)
            .expect("valid JSONPath filter should succeed");
        let query = builder.build_query();

        assert!(query.contains("r.vault = $1"));
        assert!(query.contains("r.exit_code = 0"));
        assert!(query.contains("EXISTS"));
        assert!(query.contains("ro.config"));
        assert!(query.contains("ro.output"));
    }

    // ---- ParameterMatch ------------------------------------------------

    fn bound_strings(builder: &RunQueryBuilder) -> Vec<String> {
        builder
            .bind_values()
            .iter()
            .filter_map(|v| match v {
                BindValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn pm_file_and_parameter_generate_full_query() {
        let pm = ParameterMatch {
            phase: "pre".into(),
            file: Some("config/sat1/orbit.json".into()),
            parameter: Some("a".into()),
            operator: Some(ComparisonOp::Ge),
            value: Some(serde_json::json!(1.0)),
        };
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("valid parameter match");
        let query = builder.build_query();
        assert!(query.contains("EXISTS"));
        assert!(query.contains("ro.output ? 'content'"));
        assert!(query.contains("ro.config->>'path'"));
        assert!(query.contains("jsonb_path_exists"));

        let bound = bound_strings(&builder);
        assert!(bound.contains(&"pre".to_string()));
        assert!(bound.contains(&"config/sat1/orbit.json".to_string()));
        assert!(
            bound.iter().any(|s| s == "$.content.a ? (@ >= 1.0)"),
            "expected JSONPath rooted at $.content, got bound values: {bound:?}"
        );
    }

    #[test]
    fn pm_file_only_omits_jsonpath() {
        let pm = ParameterMatch {
            phase: "pre".into(),
            file: Some("config.json".into()),
            parameter: None,
            operator: None,
            value: None,
        };
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("file-only parameter match");
        let query = builder.build_query();
        assert!(query.contains("ro.output ? 'content'"));
        assert!(query.contains("ro.config->>'path'"));
        assert!(!query.contains("jsonb_path_exists"));
    }

    #[test]
    fn pm_parameter_only_omits_file_clause() {
        let pm = ParameterMatch {
            phase: "pre".into(),
            file: None,
            parameter: Some("lr".into()),
            operator: Some(ComparisonOp::Ge),
            value: Some(serde_json::json!(0.01)),
        };
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("parameter-only match (broad)");
        let query = builder.build_query();
        assert!(query.contains("ro.output ? 'content'"));
        assert!(!query.contains("ro.config->>'path'"));
        assert!(query.contains("jsonb_path_exists"));
    }

    #[test]
    fn pm_rejects_empty_match() {
        let pm = ParameterMatch {
            phase: "pre".into(),
            file: None,
            parameter: None,
            operator: None,
            value: None,
        };
        let err = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .unwrap_err();
        assert!(format!("{err:?}").contains("at least one"));
    }

    #[test]
    fn pm_rejects_partial_triple() {
        let pm = ParameterMatch {
            phase: "pre".into(),
            file: Some("c.json".into()),
            parameter: Some("lr".into()),
            operator: Some(ComparisonOp::Ge),
            // value missing
            value: None,
        };
        let err = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .unwrap_err();
        assert!(format!("{err:?}").contains("all be specified together"));
    }

    #[test]
    fn pm_rejects_invalid_phase() {
        let pm = ParameterMatch {
            phase: "during".into(),
            file: Some("c.json".into()),
            parameter: None,
            operator: None,
            value: None,
        };
        let err = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .unwrap_err();
        assert!(format!("{err:?}").contains("phase"));
    }

    #[test]
    fn pm_rejects_invalid_parameter_name() {
        let pm = ParameterMatch {
            phase: "pre".into(),
            file: None,
            parameter: Some("x; DROP TABLE".into()),
            operator: Some(ComparisonOp::Eq),
            value: Some(serde_json::json!(1)),
        };
        let result = RunQueryBuilder::new().with_parameter_match(&pm);
        assert!(result.is_err());
    }

    #[test]
    fn pm_nested_dot_path_in_jsonpath() {
        let pm = ParameterMatch {
            phase: "post".into(),
            file: Some("results.json".into()),
            parameter: Some("metrics.max_temp".into()),
            operator: Some(ComparisonOp::Le),
            value: Some(serde_json::json!(85.0)),
        };
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("nested dot path");
        let bound = bound_strings(&builder);
        assert!(
            bound
                .iter()
                .any(|s| s == "$.content.metrics.max_temp ? (@ <= 85.0)"),
            "expected nested dot path in JSONPath, got: {bound:?}"
        );
    }

    #[test]
    fn pm_build_jsonpath_string_value_quoted() {
        let expr = build_parameter_jsonpath("orbit", &ComparisonOp::Eq, &serde_json::json!("LEO"))
            .expect("valid string match");
        assert_eq!(expr, r#"$.content.orbit ? (@ == "LEO")"#);
    }
}
