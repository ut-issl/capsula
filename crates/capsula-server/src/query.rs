//! Query builder for constructing SQL queries with JSONPath filters
//!
//! This module provides a builder for constructing dynamic SQL queries
//! that filter runs based on metadata and hook outputs using JSONPath expressions.

use crate::models::{HookFilter, SearchRunsRequest, SortOrder};
use chrono::{DateTime, Utc};

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
#[allow(dead_code)] // Some variants are reserved for future use
pub enum BindValue {
    String(String),
    I32(i32),
    I64(i64),
    DateTime(DateTime<Utc>),
    Bool(bool),
}

impl RunQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
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

    /// Add a success filter (exit_code = 0 or exit_code != 0)
    pub fn with_success(mut self, success: bool) -> Self {
        if success {
            self.conditions.push("r.exit_code = 0".to_string());
        } else {
            self.conditions
                .push("r.exit_code IS NOT NULL AND r.exit_code != 0".to_string());
        }
        self
    }

    /// Add a hook filter using JSONPath
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

    /// Validate a JSONPath expression (basic validation)
    fn validate_jsonpath(expr: &str) -> Result<(), QueryError> {
        // Basic validation: must start with $
        if !expr.starts_with('$') {
            return Err(QueryError::InvalidJsonPath(format!(
                "JSONPath must start with '$': {expr}"
            )));
        }

        // Check for obviously dangerous patterns (SQL injection attempts)
        let dangerous_patterns = ["--", ";", "/*", "*/", "DROP", "DELETE", "INSERT", "UPDATE"];
        let upper = expr.to_uppercase();
        for pattern in dangerous_patterns {
            if upper.contains(pattern) {
                return Err(QueryError::InvalidJsonPath(format!(
                    "Invalid characters in JSONPath: {expr}"
                )));
            }
        }

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

        // Add LIMIT and OFFSET
        query.push_str(&format!(
            " LIMIT {} OFFSET {}",
            self.limit.min(1000),
            self.offset
        ));

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
            .unwrap();
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
            .unwrap();
        let query = builder.build_query();

        assert!(query.contains("r.vault = $1"));
        assert!(query.contains("r.exit_code = 0"));
        assert!(query.contains("EXISTS"));
        assert!(query.contains("ro.config"));
        assert!(query.contains("ro.output"));
    }
}
