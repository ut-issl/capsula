//! Runs: the records pushed by `capsula push`. Run ids are ULIDs chosen by the
//! CLI and unique within an organization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sql_json_path::JsonPath;
use sqlx::{PgExecutor, PgPool};
use std::fmt::Write;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct Run {
    pub org_id: Uuid,
    pub id: String,
    pub vault_id: Uuid,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub project_root: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub pushed_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewRun<'a> {
    pub id: &'a str,
    pub vault_id: Uuid,
    pub name: &'a str,
    pub timestamp: DateTime<Utc>,
    pub command: &'a str,
    pub project_root: &'a str,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout: Option<&'a str>,
    pub stderr: Option<&'a str>,
    pub pushed_by_user_id: Option<Uuid>,
}

impl<'a> NewRun<'a> {
    /// A run with placeholder metadata, for tests and fixtures.
    #[must_use]
    pub const fn minimal(id: &'a str, vault_id: Uuid, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            vault_id,
            name: id,
            timestamp,
            command: "[\"true\"]",
            project_root: "/project",
            exit_code: Some(0),
            duration_ms: Some(1),
            stdout: None,
            stderr: None,
            pushed_by_user_id: None,
        }
    }
}

const COLUMNS: &str = "org_id, id, vault_id, name, timestamp, command, project_root, \
                       exit_code, duration_ms, stdout, stderr, pushed_by_user_id, \
                       created_at, updated_at";

/// Insert a run. Fails with a unique violation when `(org_id, id)` exists and
/// with a foreign-key violation when the vault does not belong to the org.
pub async fn create(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    new: &NewRun<'_>,
) -> sqlx::Result<Run> {
    sqlx::query_as!(
        Run,
        r#"
        INSERT INTO runs (
            org_id, id, vault_id, name, timestamp, command, project_root,
            exit_code, duration_ms, stdout, stderr, pushed_by_user_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING org_id, id, vault_id, name, timestamp, command, project_root,
                  exit_code, duration_ms, stdout, stderr, pushed_by_user_id,
                  created_at, updated_at
        "#,
        org_id,
        new.id,
        new.vault_id,
        new.name,
        new.timestamp,
        new.command,
        new.project_root,
        new.exit_code,
        new.duration_ms,
        new.stdout,
        new.stderr,
        new.pushed_by_user_id
    )
    .fetch_one(exec)
    .await
}

pub async fn get(exec: impl PgExecutor<'_>, org_id: Uuid, id: &str) -> sqlx::Result<Option<Run>> {
    sqlx::query_as!(
        Run,
        r#"
        SELECT org_id, id, vault_id, name, timestamp, command, project_root,
               exit_code, duration_ms, stdout, stderr, pushed_by_user_id,
               created_at, updated_at
        FROM runs
        WHERE org_id = $1 AND id = $2
        "#,
        org_id,
        id
    )
    .fetch_optional(exec)
    .await
}

/// A page of runs, newest first, optionally limited to one vault.
pub async fn list(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    vault_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<Run>> {
    sqlx::query_as!(
        Run,
        r#"
        SELECT org_id, id, vault_id, name, timestamp, command, project_root,
               exit_code, duration_ms, stdout, stderr, pushed_by_user_id,
               created_at, updated_at
        FROM runs
        WHERE org_id = $1 AND ($2::uuid IS NULL OR vault_id = $2)
        ORDER BY timestamp DESC, id DESC
        LIMIT $3 OFFSET $4
        "#,
        org_id,
        vault_id,
        limit,
        offset
    )
    .fetch_all(exec)
    .await
}

/// The number of runs, optionally limited to one vault.
pub async fn count(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    vault_id: Option<Uuid>,
) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!"
        FROM runs
        WHERE org_id = $1 AND ($2::uuid IS NULL OR vault_id = $2)
        "#,
        org_id,
        vault_id
    )
    .fetch_one(exec)
    .await
}

/// Delete a run with its hook outputs and captured-file rows. Objects in
/// storage are the caller's responsibility. Returns whether the run existed.
pub async fn delete(exec: impl PgExecutor<'_>, org_id: Uuid, id: &str) -> sqlx::Result<bool> {
    let result = sqlx::query!("DELETE FROM runs WHERE org_id = $1 AND id = $2", org_id, id)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Maximum length of a `JSONPath` expression (denial-of-service guard).
const MAX_JSONPATH_LENGTH: usize = 500;
/// Callers asking for more rows than this are clamped.
pub const MAX_LIMIT: i64 = 1_000;
/// Callers asking for a larger offset than this are clamped; a huge OFFSET
/// forces a sequential scan.
pub const MAX_OFFSET: i64 = 100_000;
const DEFAULT_LIMIT: i64 = 100;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    LatestFirst,
    OldestFirst,
}

/// A condition on one hook's config and output, as SQL/JSON path expressions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookFilter {
    /// The hook id, for example `capture-git-repo`.
    pub hook_id: String,
    /// `JSONPath` that the hook's config must satisfy, if any.
    pub config_filter: Option<String>,
    /// `JSONPath` that the hook's output must satisfy.
    pub output_filter: String,
}

/// Search criteria. All conditions are combined with AND.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunSearch {
    /// Only runs of the vault with this name.
    pub vault: Option<String>,
    /// Only runs at or after this time.
    pub from: Option<DateTime<Utc>>,
    /// Only runs at or before this time.
    pub to: Option<DateTime<Utc>>,
    /// Only runs with exactly this exit code.
    pub exit_code: Option<i32>,
    /// Only successful (`exit_code = 0`) or only failed runs.
    pub success: Option<bool>,
    pub hook_filters: Vec<HookFilter>,
    pub order: SortOrder,
    /// Defaults to 100, clamped to [`MAX_LIMIT`].
    pub limit: Option<i64>,
    /// Defaults to 0, clamped to [`MAX_OFFSET`].
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    /// Matching runs before `limit` / `offset` were applied.
    pub total: i64,
    pub runs: Vec<Run>,
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("invalid JSONPath expression: {0}")]
    InvalidJsonPath(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Search the runs of an organization.
///
/// Takes a pool because it runs a count query and a page query.
pub async fn search(
    pool: &PgPool,
    org_id: Uuid,
    criteria: &RunSearch,
) -> Result<SearchResult, SearchError> {
    let query = SearchQuery::build(org_id, criteria)?;

    // SAFETY: `SearchQuery` interpolates only server-selected SQL fragments
    // (fixed column predicates, `$n` placeholders, a fixed sort order and
    // clamped numeric LIMIT/OFFSET). Every user-provided value is bound.
    let total = query
        .bind_all(sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(
            query.count_sql(),
        )))
        .fetch_one(pool)
        .await?;
    let runs = query
        .bind_all_as(sqlx::query_as::<_, Run>(sqlx::AssertSqlSafe(
            query.page_sql(),
        )))
        .fetch_all(pool)
        .await?;

    Ok(SearchResult { total, runs })
}

#[derive(Debug, Clone)]
enum BindValue {
    Uuid(Uuid),
    String(String),
    I32(i32),
    DateTime(DateTime<Utc>),
}

/// The SQL of one search, with its bind values.
#[derive(Debug)]
struct SearchQuery {
    conditions: Vec<String>,
    bind_values: Vec<BindValue>,
    order: SortOrder,
    limit: i64,
    offset: i64,
}

impl SearchQuery {
    fn build(org_id: Uuid, criteria: &RunSearch) -> Result<Self, SearchError> {
        let mut query = Self {
            conditions: Vec::new(),
            bind_values: Vec::new(),
            order: criteria.order.clone(),
            limit: criteria.limit.unwrap_or(DEFAULT_LIMIT).clamp(0, MAX_LIMIT),
            offset: criteria.offset.unwrap_or(0).clamp(0, MAX_OFFSET),
        };

        let p = query.bind(BindValue::Uuid(org_id));
        query.conditions.push(format!("r.org_id = {p}"));

        if let Some(vault) = &criteria.vault {
            let p = query.bind(BindValue::String(vault.clone()));
            query.conditions.push(format!("v.name = {p}"));
        }
        if let Some(from) = criteria.from {
            let p = query.bind(BindValue::DateTime(from));
            query.conditions.push(format!("r.timestamp >= {p}"));
        }
        if let Some(to) = criteria.to {
            let p = query.bind(BindValue::DateTime(to));
            query.conditions.push(format!("r.timestamp <= {p}"));
        }
        if let Some(exit_code) = criteria.exit_code {
            let p = query.bind(BindValue::I32(exit_code));
            query.conditions.push(format!("r.exit_code = {p}"));
        }
        match criteria.success {
            Some(true) => query.conditions.push("r.exit_code = 0".to_owned()),
            Some(false) => query
                .conditions
                .push("r.exit_code IS NOT NULL AND r.exit_code <> 0".to_owned()),
            None => {}
        }
        for filter in &criteria.hook_filters {
            query.add_hook_filter(filter)?;
        }
        Ok(query)
    }

    /// Register a bind value and return its `$n` placeholder.
    fn bind(&mut self, value: BindValue) -> String {
        self.bind_values.push(value);
        format!("${}", self.bind_values.len())
    }

    fn add_hook_filter(&mut self, filter: &HookFilter) -> Result<(), SearchError> {
        validate_jsonpath(&filter.output_filter)?;
        if let Some(config_filter) = &filter.config_filter {
            validate_jsonpath(config_filter)?;
        }

        let mut conditions = vec![
            "ro.org_id = r.org_id".to_owned(),
            "ro.run_id = r.id".to_owned(),
        ];
        let p = self.bind(BindValue::String(filter.hook_id.clone()));
        conditions.push(format!("ro.hook_id = {p}"));
        if let Some(config_filter) = &filter.config_filter {
            let p = self.bind(BindValue::String(config_filter.clone()));
            conditions.push(format!("jsonb_path_exists(ro.config, {p}::jsonpath)"));
        }
        let p = self.bind(BindValue::String(filter.output_filter.clone()));
        conditions.push(format!("jsonb_path_exists(ro.output, {p}::jsonpath)"));

        self.conditions.push(format!(
            "EXISTS (SELECT 1 FROM run_outputs ro WHERE {})",
            conditions.join(" AND ")
        ));
        Ok(())
    }

    fn source_and_filter_sql(&self) -> String {
        format!(
            " FROM runs r JOIN vaults v ON v.org_id = r.org_id AND v.id = r.vault_id WHERE {}",
            self.conditions.join(" AND ")
        )
    }

    fn count_sql(&self) -> String {
        format!("SELECT COUNT(*){}", self.source_and_filter_sql())
    }

    fn page_sql(&self) -> String {
        let mut sql = String::from("SELECT ");
        sql.push_str(
            &COLUMNS
                .split(", ")
                .map(|c| format!("r.{}", c.trim()))
                .collect::<Vec<_>>()
                .join(", "),
        );
        sql.push_str(&self.source_and_filter_sql());
        match self.order {
            SortOrder::LatestFirst => sql.push_str(" ORDER BY r.timestamp DESC, r.id DESC"),
            SortOrder::OldestFirst => sql.push_str(" ORDER BY r.timestamp ASC, r.id ASC"),
        }
        let _ = write!(sql, " LIMIT {} OFFSET {}", self.limit, self.offset);
        sql
    }

    fn bind_all<'q, O>(
        &'q self,
        mut query: sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
        for value in &self.bind_values {
            query = match value {
                BindValue::Uuid(v) => query.bind(*v),
                BindValue::String(v) => query.bind(v.as_str()),
                BindValue::I32(v) => query.bind(*v),
                BindValue::DateTime(v) => query.bind(*v),
            };
        }
        query
    }

    fn bind_all_as<'q, O>(
        &'q self,
        mut query: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
        for value in &self.bind_values {
            query = match value {
                BindValue::Uuid(v) => query.bind(*v),
                BindValue::String(v) => query.bind(v.as_str()),
                BindValue::I32(v) => query.bind(*v),
                BindValue::DateTime(v) => query.bind(*v),
            };
        }
        query
    }
}

/// Validate a SQL/JSON path expression with a parser compatible with
/// PostgreSQL's (`starts with`, `like_regex`, `.type()`, arithmetic, ...).
fn validate_jsonpath(expr: &str) -> Result<(), SearchError> {
    if expr.len() > MAX_JSONPATH_LENGTH {
        return Err(SearchError::InvalidJsonPath(format!(
            "expression longer than {MAX_JSONPATH_LENGTH} characters"
        )));
    }
    JsonPath::new(expr).map_err(|e| SearchError::InvalidJsonPath(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_outputs::{self, NewRunOutput, Phase};
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::{is_foreign_key_violation, is_unique_violation, vaults};
    use chrono::TimeZone;
    use serde_json::json;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 8, hour, 0, 0).unwrap()
    }

    #[tokio::test]
    async fn create_get_list_count_delete() {
        let pool = test_pool().await;
        let (alice, org) = user_with_org(&pool, "alice").await;
        let (_, other) = user_with_org(&pool, "bob").await;
        let vault = vaults::create(&pool, org.id, "sim").await.unwrap();
        let other_vault = vaults::create(&pool, org.id, "analysis").await.unwrap();
        let bobs_vault = vaults::create(&pool, other.id, "sim").await.unwrap();

        let new = NewRun {
            id: "01RUN",
            vault_id: vault.id,
            name: "chubby-back",
            timestamp: at(10),
            command: "[\"cargo\",\"test\"]",
            project_root: "/home/alice/proj",
            exit_code: Some(0),
            duration_ms: Some(1234),
            stdout: Some("ok"),
            stderr: None,
            pushed_by_user_id: Some(alice.id),
        };
        let run = create(&pool, org.id, &new).await.unwrap();
        assert_eq!(run.id, "01RUN");
        assert_eq!(run.pushed_by_user_id, Some(alice.id));
        assert_eq!(run.stdout.as_deref(), Some("ok"));

        let err = create(&pool, org.id, &new).await.unwrap_err();
        assert!(is_unique_violation(&err), "{err}");
        // The same id is free in another org.
        create(
            &pool,
            other.id,
            &NewRun::minimal("01RUN", bobs_vault.id, at(10)),
        )
        .await
        .unwrap();
        // A vault of another org cannot be used.
        let err = create(
            &pool,
            org.id,
            &NewRun::minimal("01BAD", bobs_vault.id, at(10)),
        )
        .await
        .unwrap_err();
        assert!(is_foreign_key_violation(&err), "{err}");

        create(&pool, org.id, &NewRun::minimal("01OLD", vault.id, at(8)))
            .await
            .unwrap();
        create(
            &pool,
            org.id,
            &NewRun::minimal("01NEW", other_vault.id, at(12)),
        )
        .await
        .unwrap();

        assert_eq!(
            get(&pool, org.id, "01RUN").await.unwrap(),
            Some(run.clone())
        );
        assert_eq!(get(&pool, other.id, "01OLD").await.unwrap(), None);

        let ids = |runs: Vec<Run>| runs.into_iter().map(|r| r.id).collect::<Vec<_>>();
        assert_eq!(
            ids(list(&pool, org.id, None, 10, 0).await.unwrap()),
            ["01NEW", "01RUN", "01OLD"]
        );
        assert_eq!(
            ids(list(&pool, org.id, None, 1, 1).await.unwrap()),
            ["01RUN"]
        );
        assert_eq!(
            ids(list(&pool, org.id, Some(vault.id), 10, 0).await.unwrap()),
            ["01RUN", "01OLD"]
        );
        assert_eq!(count(&pool, org.id, None).await.unwrap(), 3);
        assert_eq!(count(&pool, org.id, Some(vault.id)).await.unwrap(), 2);
        assert_eq!(count(&pool, other.id, None).await.unwrap(), 1);

        assert!(delete(&pool, org.id, "01RUN").await.unwrap());
        assert!(!delete(&pool, org.id, "01RUN").await.unwrap());
        assert!(!delete(&pool, other.id, "01OLD").await.unwrap());
        assert_eq!(count(&pool, org.id, None).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn deleting_the_pusher_keeps_the_run() {
        let pool = test_pool().await;
        let (alice, org) = user_with_org(&pool, "alice").await;
        let vault = vaults::create(&pool, org.id, "sim").await.unwrap();
        let mut new = NewRun::minimal("01RUN", vault.id, at(10));
        new.pushed_by_user_id = Some(alice.id);
        create(&pool, org.id, &new).await.unwrap();

        crate::db::users::delete(&pool, alice.id).await.unwrap();
        let run = get(&pool, org.id, "01RUN").await.unwrap().unwrap();
        assert_eq!(run.pushed_by_user_id, None);
    }

    async fn seed_search_fixture(pool: &PgPool) -> (Uuid, Uuid) {
        let (_, org) = user_with_org(pool, "alice").await;
        let (_, other) = user_with_org(pool, "bob").await;
        let sim = vaults::create(pool, org.id, "sim").await.unwrap();
        let analysis = vaults::create(pool, org.id, "analysis").await.unwrap();
        let bobs = vaults::create(pool, other.id, "sim").await.unwrap();

        let mut ok = NewRun::minimal("01OK", sim.id, at(10));
        ok.exit_code = Some(0);
        let mut failed = NewRun::minimal("01FAIL", sim.id, at(11));
        failed.exit_code = Some(1);
        let mut running = NewRun::minimal("01RUNNING", analysis.id, at(12));
        running.exit_code = None;
        for new in [&ok, &failed, &running] {
            create(pool, org.id, new).await.unwrap();
        }
        create(pool, other.id, &NewRun::minimal("01BOB", bobs.id, at(10)))
            .await
            .unwrap();

        for (run_id, sha, env_value) in [
            ("01OK", "abc123", "production"),
            ("01FAIL", "a1b999", "staging"),
        ] {
            run_outputs::upsert(
                pool,
                org.id,
                run_id,
                &NewRunOutput {
                    phase: Phase::Pre,
                    hook_index: 0,
                    hook_id: "capture-git-repo",
                    output: &json!({ "sha": sha, "dirty": false }),
                    config: Some(&json!({ "path": "." })),
                    success: true,
                    error: None,
                    failure_reason: None,
                },
            )
            .await
            .unwrap();
            run_outputs::upsert(
                pool,
                org.id,
                run_id,
                &NewRunOutput {
                    phase: Phase::Pre,
                    hook_index: 1,
                    hook_id: "capture-env",
                    output: &json!({ "value": env_value }),
                    config: Some(&json!({ "name": "ENV" })),
                    success: true,
                    error: None,
                    failure_reason: None,
                },
            )
            .await
            .unwrap();
        }
        // Bob has a matching hook output too; it must never show up for Alice.
        run_outputs::upsert(
            pool,
            other.id,
            "01BOB",
            &NewRunOutput {
                phase: Phase::Pre,
                hook_index: 0,
                hook_id: "capture-git-repo",
                output: &json!({ "sha": "abc123" }),
                config: None,
                success: true,
                error: None,
                failure_reason: None,
            },
        )
        .await
        .unwrap();
        (org.id, other.id)
    }

    fn ids(result: &SearchResult) -> Vec<&str> {
        result.runs.iter().map(|r| r.id.as_str()).collect()
    }

    #[tokio::test]
    async fn search_filters_and_pages() {
        let pool = test_pool().await;
        let (org, other) = seed_search_fixture(&pool).await;

        let all = search(&pool, org, &RunSearch::default()).await.unwrap();
        assert_eq!(all.total, 3);
        assert_eq!(ids(&all), ["01RUNNING", "01FAIL", "01OK"]);

        let oldest = search(
            &pool,
            org,
            &RunSearch {
                order: SortOrder::OldestFirst,
                limit: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(oldest.total, 3);
        assert_eq!(ids(&oldest), ["01OK", "01FAIL"]);

        let page2 = search(
            &pool,
            org,
            &RunSearch {
                limit: Some(2),
                offset: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&page2), ["01OK"]);

        let in_sim = search(
            &pool,
            org,
            &RunSearch {
                vault: Some("sim".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&in_sim), ["01FAIL", "01OK"]);

        let window = search(
            &pool,
            org,
            &RunSearch {
                from: Some(at(11)),
                to: Some(at(11)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&window), ["01FAIL"]);

        let failed = search(
            &pool,
            org,
            &RunSearch {
                success: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&failed), ["01FAIL"]);
        let succeeded = search(
            &pool,
            org,
            &RunSearch {
                success: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&succeeded), ["01OK"]);
        let code1 = search(
            &pool,
            org,
            &RunSearch {
                exit_code: Some(1),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&code1), ["01FAIL"]);

        // Another org's runs are invisible even without filters.
        let bobs = search(&pool, other, &RunSearch::default()).await.unwrap();
        assert_eq!(ids(&bobs), ["01BOB"]);
    }

    #[tokio::test]
    async fn search_by_hook_output_and_config() {
        let pool = test_pool().await;
        let (org, _) = seed_search_fixture(&pool).await;

        let sha_prefix = search(
            &pool,
            org,
            &RunSearch {
                hook_filters: vec![HookFilter {
                    hook_id: "capture-git-repo".to_owned(),
                    config_filter: None,
                    output_filter: "$.sha ? (@ starts with \"abc\")".to_owned(),
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&sha_prefix), ["01OK"]);

        let env = search(
            &pool,
            org,
            &RunSearch {
                hook_filters: vec![HookFilter {
                    hook_id: "capture-env".to_owned(),
                    config_filter: Some("$.name ? (@ == \"ENV\")".to_owned()),
                    output_filter: "$.value ? (@ == \"staging\")".to_owned(),
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(ids(&env), ["01FAIL"]);

        // Filters combine with AND.
        let none = search(
            &pool,
            org,
            &RunSearch {
                hook_filters: vec![
                    HookFilter {
                        hook_id: "capture-git-repo".to_owned(),
                        config_filter: None,
                        output_filter: "$.sha ? (@ starts with \"abc\")".to_owned(),
                    },
                    HookFilter {
                        hook_id: "capture-env".to_owned(),
                        config_filter: None,
                        output_filter: "$.value ? (@ == \"staging\")".to_owned(),
                    },
                ],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(none.total, 0);
        assert_eq!(none.runs, vec![]);

        // The config filter must match the same hook row, not another hook's config.
        let wrong_hook = search(
            &pool,
            org,
            &RunSearch {
                hook_filters: vec![HookFilter {
                    hook_id: "capture-git-repo".to_owned(),
                    config_filter: Some("$.name ? (@ == \"ENV\")".to_owned()),
                    output_filter: "$.sha".to_owned(),
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(wrong_hook.total, 0);
    }

    #[tokio::test]
    async fn search_rejects_invalid_jsonpath() {
        let pool = test_pool().await;
        let (org, _) = seed_search_fixture(&pool).await;
        for bad in ["not-a-path", &"$.a".repeat(200)] {
            let err = search(
                &pool,
                org,
                &RunSearch {
                    hook_filters: vec![HookFilter {
                        hook_id: "capture-env".to_owned(),
                        config_filter: None,
                        output_filter: bad.to_owned(),
                    }],
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(err, SearchError::InvalidJsonPath(_)), "{err}");
        }
    }

    #[test]
    fn limit_and_offset_are_clamped() {
        let query = SearchQuery::build(
            Uuid::nil(),
            &RunSearch {
                limit: Some(1_000_000),
                offset: Some(-5),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(query.page_sql().ends_with("LIMIT 1000 OFFSET 0"));
        let query = SearchQuery::build(Uuid::nil(), &RunSearch::default()).unwrap();
        assert!(query.page_sql().ends_with("LIMIT 100 OFFSET 0"));
        assert_eq!(query.bind_values.len(), 1);
    }
}
