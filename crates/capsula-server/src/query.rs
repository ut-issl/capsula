//! Query builder for constructing SQL queries with `JSONPath` filters
//!
//! This module provides a builder for constructing dynamic SQL queries
//! that filter runs based on metadata and hook outputs using `JSONPath` expressions.

use crate::models::{
    ComparisonOp, HookFilter, ParameterCondition, ParameterMatch, SearchRunsRequest, SortOrder,
};
use chrono::{DateTime, Utc};
use sql_json_path::JsonPath;
use std::fmt::Write;

/// `str::len` cap for `JSONPath` expressions passed to `sql-json-path`.
const MAX_JSONPATH_LENGTH: usize = 500;

const MAX_LIMIT: i64 = 1_000;

/// Guards against a large `OFFSET` forcing a sequential scan.
const MAX_OFFSET: i64 = 100_000;

/// Each entry becomes an independent `EXISTS`, so an unbounded list
/// would let a caller stall the planner.
const MAX_PARAMETER_MATCHES: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Invalid JSONPath expression: {0}")]
    InvalidJsonPath(String),
    #[error("Invalid parameter match: {0}")]
    InvalidParameterMatch(String),
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

        if request.parameter_matches.len() > MAX_PARAMETER_MATCHES {
            return Err(QueryError::InvalidParameterMatch(format!(
                "at most {MAX_PARAMETER_MATCHES} parameter_matches entries are allowed, got {}",
                request.parameter_matches.len()
            )));
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

    /// Add a `ParameterMatch` filter — targets rows produced by
    /// parameter-capturing hooks and combines phase / file / `hook_index`
    /// pins with an optional multi-condition `JSONPath` predicate.
    pub fn with_parameter_match(mut self, pm: &ParameterMatch) -> Result<Self, QueryError> {
        // At least one of file / hook_index / conditions is required —
        // an unconstrained match would scan every parameter-capturing
        // row of the run.
        if pm.file.is_none() && pm.hook_index.is_none() && pm.conditions.is_empty() {
            return Err(QueryError::InvalidParameterMatch(
                "ParameterMatch requires at least one of 'file', 'hook_index', \
                 or a non-empty 'conditions'"
                    .to_string(),
            ));
        }

        // `hook_index` is `i32` for parity with the DB column but the
        // API contract is non-negative; reject negatives so callers see
        // a clear error instead of an empty result set.
        if let Some(hook_index) = pm.hook_index
            && hook_index < 0
        {
            return Err(QueryError::InvalidParameterMatch(format!(
                "hook_index must be non-negative, got {hook_index}"
            )));
        }

        // Ordering operators on booleans have no defined JSONPath
        // semantics — reject at the input layer instead of letting
        // Postgres error out.
        for c in &pm.conditions {
            if matches!(c.value, serde_json::Value::Bool(_))
                && matches!(
                    c.operator,
                    ComparisonOp::Gt | ComparisonOp::Ge | ComparisonOp::Lt | ComparisonOp::Le
                )
            {
                return Err(QueryError::InvalidParameterMatch(
                    "boolean value only supports 'eq' / 'ne' operators".to_string(),
                ));
            }
        }

        if !pm.conditions.is_empty() && pm.file.is_none() && pm.hook_index.is_none() {
            tracing::warn!(
                "ParameterMatch without 'file' or 'hook_index' will match across \
                 every parameter-capturing row of the run; specify one to narrow"
            );
        }

        let mut conds = vec![
            "ro.run_id = r.id".to_string(),
            format!("ro.phase = ${}", self.param_index),
        ];
        self.bind_values
            .push(BindValue::String(pm.phase.as_str().to_string()));
        self.param_index += 1;

        // Structural filter: only parameter-capturing rows have `content`.
        conds.push("ro.output ? 'content'".to_string());

        if let Some(file) = &pm.file {
            conds.push(format!("ro.config->>'path' = ${}", self.param_index));
            self.bind_values.push(BindValue::String(file.clone()));
            self.param_index += 1;
        }

        if let Some(hook_index) = pm.hook_index {
            conds.push(format!("ro.hook_index = ${}", self.param_index));
            self.bind_values.push(BindValue::I32(hook_index));
            self.param_index += 1;
        }

        if !pm.conditions.is_empty() {
            let jsonpath = build_conditions_jsonpath(&pm.conditions)?;
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

/// Compile a list of conditions into a single `JSONPath` predicate.
///
/// One condition emits the compact `$.content.<path> ? (@ <op> <val>)`
/// form so single-parameter matches stay legible. Two or more emit the
/// composable `$ ? (@.content.<p1> <op1> <v1> && ...)` form — the
/// per-row AND that motivated putting conditions inside one entry
/// (see `ParameterMatch::conditions` for the false-positive it avoids).
fn build_conditions_jsonpath(conditions: &[ParameterCondition]) -> Result<String, QueryError> {
    debug_assert!(
        !conditions.is_empty(),
        "caller must not pass empty conditions"
    );

    if conditions.len() == 1 {
        let c = &conditions[0];
        let path = build_path(&c.parameter)?;
        let op = op_to_jsonpath(&c.operator);
        let val = value_to_jsonpath(&c.value)?;
        return Ok(format!("$.content.{path} ? (@ {op} {val})"));
    }

    let mut parts = Vec::with_capacity(conditions.len());
    for c in conditions {
        let path = build_path(&c.parameter)?;
        let op = op_to_jsonpath(&c.operator);
        let val = value_to_jsonpath(&c.value)?;
        parts.push(format!("@.content.{path} {op} {val}"));
    }
    Ok(format!("$ ? ({})", parts.join(" && ")))
}

/// Build the dot-path portion of a `JSONPath`, quoting segments that
/// contain characters outside the bare-identifier set `[A-Za-z0-9_]`.
///
/// Quoting exists so keys with hyphens (`learning-rate`), spaces, or
/// non-ASCII characters (`温度`) remain addressable without inviting
/// injection — control characters and empty segments are still
/// rejected, and `\` / `"` inside a segment are escaped.
fn build_path(parameter: &str) -> Result<String, QueryError> {
    if parameter.is_empty() {
        return Err(QueryError::InvalidParameterMatch(
            "parameter must not be empty".to_string(),
        ));
    }
    if parameter.len() > 200 {
        return Err(QueryError::InvalidParameterMatch(
            "parameter must be at most 200 bytes".to_string(),
        ));
    }

    let mut segments = Vec::new();
    for seg in parameter.split('.') {
        if seg.is_empty() {
            return Err(QueryError::InvalidParameterMatch(
                "parameter has an empty segment (leading, trailing, or consecutive '.')"
                    .to_string(),
            ));
        }
        for ch in seg.chars() {
            if ch.is_control() {
                return Err(QueryError::InvalidParameterMatch(format!(
                    "parameter segment contains a control character: {ch:?}"
                )));
            }
        }
        segments.push(quote_segment_if_needed(seg));
    }
    Ok(segments.join("."))
}

fn quote_segment_if_needed(seg: &str) -> String {
    if seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        seg.to_string()
    } else {
        format!("\"{}\"", seg.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

const fn op_to_jsonpath(op: &ComparisonOp) -> &'static str {
    match op {
        ComparisonOp::Eq => "==",
        ComparisonOp::Ne => "!=",
        ComparisonOp::Gt => ">",
        ComparisonOp::Ge => ">=",
        ComparisonOp::Lt => "<",
        ComparisonOp::Le => "<=",
    }
}

fn value_to_jsonpath(value: &serde_json::Value) -> Result<String, QueryError> {
    match value {
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(format!(
            "\"{}\"",
            s.replace('\\', "\\\\").replace('"', "\\\"")
        )),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        _ => Err(QueryError::InvalidParameterMatch(
            "value must be number, string, or boolean".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Phase;

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

    fn cond(
        parameter: &str,
        operator: ComparisonOp,
        value: serde_json::Value,
    ) -> ParameterCondition {
        ParameterCondition {
            parameter: parameter.to_string(),
            operator,
            value,
        }
    }

    fn pm_with(
        phase: Phase,
        file: Option<&str>,
        hook_index: Option<i32>,
        conditions: Vec<ParameterCondition>,
    ) -> ParameterMatch {
        ParameterMatch {
            phase,
            file: file.map(str::to_string),
            hook_index,
            conditions,
        }
    }

    #[test]
    fn pm_file_and_single_condition_generate_full_query() {
        let pm = pm_with(
            Phase::Pre,
            Some("config/sat1/orbit.json"),
            None,
            vec![cond("a", ComparisonOp::Ge, serde_json::json!(1.0))],
        );
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
            "expected single-condition compact form, got: {bound:?}"
        );
    }

    #[test]
    fn pm_file_only_omits_jsonpath() {
        let pm = pm_with(Phase::Pre, Some("config.json"), None, vec![]);
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("file-only parameter match");
        let query = builder.build_query();
        assert!(query.contains("ro.output ? 'content'"));
        assert!(query.contains("ro.config->>'path'"));
        assert!(!query.contains("jsonb_path_exists"));
    }

    #[test]
    fn pm_condition_only_omits_file_clause() {
        let pm = pm_with(
            Phase::Pre,
            None,
            None,
            vec![cond("lr", ComparisonOp::Ge, serde_json::json!(0.01))],
        );
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("condition-only match (broad)");
        let query = builder.build_query();
        assert!(query.contains("ro.output ? 'content'"));
        assert!(!query.contains("ro.config->>'path'"));
        assert!(query.contains("jsonb_path_exists"));
    }

    #[test]
    fn pm_rejects_empty_match() {
        let pm = pm_with(Phase::Pre, None, None, vec![]);
        let err = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .unwrap_err();
        assert!(format!("{err:?}").contains("at least one"));
    }

    #[test]
    fn pm_rejects_invalid_phase() {
        // Invalid phase is rejected by `Phase`'s serde impl before the
        // request reaches the builder — locks in that no unknown string
        // can flow through.
        let json = serde_json::json!({ "phase": "during", "file": "c.json" });
        let err = serde_json::from_value::<ParameterMatch>(json).unwrap_err();
        assert!(format!("{err}").contains("phase") || format!("{err}").contains("variant"));
    }

    #[test]
    fn pm_rejects_negative_hook_index() {
        let pm = pm_with(Phase::Pre, None, Some(-1), vec![]);
        let err = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .unwrap_err();
        assert!(format!("{err:?}").contains("non-negative"));
    }

    #[test]
    fn pm_rejects_bool_value_with_ordering_operator() {
        for op in [
            ComparisonOp::Gt,
            ComparisonOp::Ge,
            ComparisonOp::Lt,
            ComparisonOp::Le,
        ] {
            let pm = pm_with(
                Phase::Pre,
                Some("flags.json"),
                None,
                vec![cond("enabled", op, serde_json::json!(true))],
            );
            let err = RunQueryBuilder::new()
                .with_parameter_match(&pm)
                .unwrap_err();
            assert!(format!("{err:?}").contains("boolean value only supports"));
        }
    }

    #[test]
    fn pm_accepts_bool_value_with_eq_and_ne() {
        for op in [ComparisonOp::Eq, ComparisonOp::Ne] {
            let pm = pm_with(
                Phase::Pre,
                Some("flags.json"),
                None,
                vec![cond("enabled", op, serde_json::json!(true))],
            );
            assert!(RunQueryBuilder::new().with_parameter_match(&pm).is_ok());
        }
    }

    #[test]
    fn pm_rejects_too_many_parameter_matches() {
        let matches: Vec<ParameterMatch> = (0..=MAX_PARAMETER_MATCHES)
            .map(|_| pm_with(Phase::Pre, Some("c.json"), None, vec![]))
            .collect();
        let request = SearchRunsRequest {
            vault: None,
            from: None,
            to: None,
            exit_code: None,
            success: None,
            hook_filters: Vec::new(),
            parameter_matches: matches,
            include: Vec::new(),
            order: SortOrder::LatestFirst,
            limit: None,
            offset: None,
        };
        let err = RunQueryBuilder::from_request(&request).unwrap_err();
        assert!(format!("{err:?}").contains("at most"));
    }

    // ---- multi-condition (Issue 2 fix) -----------------------------

    #[test]
    fn pm_multi_condition_range_on_same_field_compiles_to_single_predicate() {
        // The whole point of `conditions` being a list: both bounds
        // apply to the *same* row, unlike two separate `parameter_matches`.
        let pm = pm_with(
            Phase::Pre,
            Some("train.json"),
            None,
            vec![
                cond("lr", ComparisonOp::Ge, serde_json::json!(0.005)),
                cond("lr", ComparisonOp::Le, serde_json::json!(0.05)),
            ],
        );
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("valid multi-condition");
        let bound = bound_strings(&builder);
        assert!(
            bound
                .iter()
                .any(|s| s == "$ ? (@.content.lr >= 0.005 && @.content.lr <= 0.05)"),
            "expected composable multi-condition form, got: {bound:?}"
        );
    }

    #[test]
    fn pm_multi_condition_different_fields_compose() {
        let pm = pm_with(
            Phase::Pre,
            Some("train.json"),
            None,
            vec![
                cond("lr", ComparisonOp::Ge, serde_json::json!(0.005)),
                cond("epoch", ComparisonOp::Eq, serde_json::json!(10)),
            ],
        );
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("valid multi-condition");
        let bound = bound_strings(&builder);
        assert!(
            bound
                .iter()
                .any(|s| s == "$ ? (@.content.lr >= 0.005 && @.content.epoch == 10)"),
            "expected multi-field compose form, got: {bound:?}"
        );
    }

    // ---- parameter path validation & quoting (Issue 4 fix) ---------

    #[test]
    fn pm_accepts_hyphenated_parameter_via_quoting() {
        let expr = build_conditions_jsonpath(&[cond(
            "learning-rate",
            ComparisonOp::Eq,
            serde_json::json!(0.1),
        )])
        .expect("hyphens must be quoted, not rejected");
        assert_eq!(expr, r#"$.content."learning-rate" ? (@ == 0.1)"#);
        JsonPath::new(&expr).expect("quoted-segment path must be valid JSONPath");
    }

    #[test]
    fn pm_accepts_non_ascii_parameter_via_quoting() {
        let expr =
            build_conditions_jsonpath(&[cond("温度", ComparisonOp::Eq, serde_json::json!(25))])
                .expect("non-ASCII keys must be quoted, not rejected");
        assert_eq!(expr, r#"$.content."温度" ? (@ == 25)"#);
        JsonPath::new(&expr).expect("quoted-segment path must be valid JSONPath");
    }

    #[test]
    fn pm_nested_path_mixes_bare_and_quoted_segments() {
        let expr = build_conditions_jsonpath(&[cond(
            "sat1.learning-rate.value",
            ComparisonOp::Le,
            serde_json::json!(0.5),
        )])
        .expect("mixed-segment path");
        assert_eq!(expr, r#"$.content.sat1."learning-rate".value ? (@ <= 0.5)"#);
    }

    #[test]
    fn pm_rejects_empty_parameter() {
        let err = build_conditions_jsonpath(&[cond("", ComparisonOp::Eq, serde_json::json!(1))])
            .unwrap_err();
        assert!(format!("{err:?}").contains("must not be empty"));
    }

    #[test]
    fn pm_rejects_empty_parameter_segment() {
        for bad in ["a..b", ".a", "a."] {
            let err =
                build_conditions_jsonpath(&[cond(bad, ComparisonOp::Eq, serde_json::json!(1))])
                    .unwrap_err();
            assert!(format!("{err:?}").contains("empty segment"), "for {bad}");
        }
    }

    #[test]
    fn pm_rejects_control_char_in_parameter() {
        let err =
            build_conditions_jsonpath(&[cond("a\nb", ComparisonOp::Eq, serde_json::json!(1))])
                .unwrap_err();
        assert!(format!("{err:?}").contains("control character"));
    }

    #[test]
    fn pm_rejects_oversize_parameter() {
        let long = "a".repeat(201);
        let err = build_conditions_jsonpath(&[cond(&long, ComparisonOp::Eq, serde_json::json!(1))])
            .unwrap_err();
        assert!(format!("{err:?}").contains("200 bytes"));
    }

    #[test]
    fn pm_quoted_segment_escapes_backslash_and_quote() {
        // A parameter segment containing both `"` and `\` must round-trip
        // through the parser rather than break the JSONPath syntax.
        let expr =
            build_conditions_jsonpath(&[cond(r#"a"b\c"#, ComparisonOp::Eq, serde_json::json!(1))])
                .expect("segment with quotes must be quoted, not rejected");
        assert_eq!(expr, r#"$.content."a\"b\\c" ? (@ == 1)"#);
        JsonPath::new(&expr).expect("escaped quoted segment must be valid JSONPath");
    }

    // ---- value validation ------------------------------------------

    #[test]
    fn pm_string_value_is_quoted_and_escaped() {
        let expr = build_conditions_jsonpath(&[cond(
            "name",
            ComparisonOp::Eq,
            serde_json::json!(r#"a"b\c"#),
        )])
        .expect("escape should succeed");
        assert_eq!(expr, r#"$.content.name ? (@ == "a\"b\\c")"#);
        JsonPath::new(&expr).expect("emitted expression must be valid JSONPath");
    }

    #[test]
    fn pm_rejects_null_value() {
        let err =
            build_conditions_jsonpath(&[cond("x", ComparisonOp::Eq, serde_json::Value::Null)])
                .unwrap_err();
        assert!(format!("{err:?}").contains("must be number, string, or boolean"));
    }

    #[test]
    fn pm_rejects_array_value() {
        let err =
            build_conditions_jsonpath(&[cond("x", ComparisonOp::Eq, serde_json::json!([1, 2]))])
                .unwrap_err();
        assert!(format!("{err:?}").contains("must be number, string, or boolean"));
    }

    // ---- hook_index composition ------------------------------------

    #[test]
    fn pm_hook_index_only_generates_hook_index_clause() {
        let pm = pm_with(Phase::Pre, None, Some(2), vec![]);
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("hook_index-only parameter match");
        let query = builder.build_query();
        assert!(query.contains("ro.output ? 'content'"));
        assert!(query.contains("ro.hook_index = $"));
        assert!(!query.contains("ro.config->>'path'"));
        assert!(!query.contains("jsonb_path_exists"));
        assert!(
            builder
                .bind_values()
                .iter()
                .any(|v| matches!(v, BindValue::I32(2)))
        );
    }

    #[test]
    fn pm_hook_index_with_file_and_conditions_composes_all_clauses() {
        let pm = pm_with(
            Phase::Post,
            Some("config.json"),
            Some(1),
            vec![cond("lr", ComparisonOp::Eq, serde_json::json!(0.01))],
        );
        let builder = RunQueryBuilder::new()
            .with_parameter_match(&pm)
            .expect("hook_index + file + conditions compose");
        let query = builder.build_query();
        assert!(query.contains("ro.output ? 'content'"));
        assert!(query.contains("ro.config->>'path'"));
        assert!(query.contains("ro.hook_index = $"));
        assert!(query.contains("jsonb_path_exists"));
    }

    #[test]
    fn pm_hook_index_alone_satisfies_at_least_one_rule() {
        let pm = pm_with(Phase::Pre, None, Some(0), vec![]);
        assert!(RunQueryBuilder::new().with_parameter_match(&pm).is_ok());
    }
}
