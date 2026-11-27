//! Hook outputs of a run, one row per hook invocation.

use super::decode_enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Pre,
    Post,
}

impl Phase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pre => "pre",
            Self::Post => "post",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pre" => Some(Self::Pre),
            "post" => Some(Self::Post),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunOutput {
    pub id: i64,
    pub org_id: Uuid,
    pub run_id: String,
    pub phase: Phase,
    /// Position of the hook in the phase's `hooks` array of `capsula.toml`.
    pub hook_index: i32,
    pub hook_id: String,
    pub output: JsonValue,
    pub config: Option<JsonValue>,
    pub success: bool,
    pub error: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewRunOutput<'a> {
    pub phase: Phase,
    pub hook_index: i32,
    pub hook_id: &'a str,
    pub output: &'a JsonValue,
    pub config: Option<&'a JsonValue>,
    pub success: bool,
    pub error: Option<&'a str>,
    pub failure_reason: Option<&'a str>,
}

struct Row {
    id: i64,
    org_id: Uuid,
    run_id: String,
    phase: String,
    hook_index: i32,
    hook_id: String,
    output: JsonValue,
    config: Option<JsonValue>,
    success: bool,
    error: Option<String>,
    failure_reason: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<Row> for RunOutput {
    type Error = sqlx::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            org_id: row.org_id,
            run_id: row.run_id,
            phase: decode_enum("phase", &row.phase, Phase::parse)?,
            hook_index: row.hook_index,
            hook_id: row.hook_id,
            output: row.output,
            config: row.config,
            success: row.success,
            error: row.error,
            failure_reason: row.failure_reason,
            created_at: row.created_at,
        })
    }
}

/// Insert a hook output, replacing the row with the same `(phase, hook_index)`.
///
/// Pushing a run twice therefore does not duplicate anything. Fails with a
/// foreign-key violation when the run does not exist in the org.
pub async fn upsert(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
    new: &NewRunOutput<'_>,
) -> sqlx::Result<RunOutput> {
    sqlx::query_as!(
        Row,
        r#"
        INSERT INTO run_outputs (
            org_id, run_id, phase, hook_index, hook_id, output, config,
            success, error, failure_reason
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (org_id, run_id, phase, hook_index) DO UPDATE
        SET hook_id = EXCLUDED.hook_id,
            output = EXCLUDED.output,
            config = EXCLUDED.config,
            success = EXCLUDED.success,
            error = EXCLUDED.error,
            failure_reason = EXCLUDED.failure_reason
        RETURNING id, org_id, run_id, phase, hook_index, hook_id, output, config,
                  success, error, failure_reason, created_at
        "#,
        org_id,
        run_id,
        new.phase.as_str(),
        new.hook_index,
        new.hook_id,
        new.output,
        new.config,
        new.success,
        new.error,
        new.failure_reason
    )
    .fetch_one(exec)
    .await?
    .try_into()
}

/// All hook outputs of a run: the pre-run phase first, each in array order.
pub async fn list(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
) -> sqlx::Result<Vec<RunOutput>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, org_id, run_id, phase, hook_index, hook_id, output, config,
               success, error, failure_reason, created_at
        FROM run_outputs
        WHERE org_id = $1 AND run_id = $2
        ORDER BY (phase = 'post'), hook_index
        "#,
        org_id,
        run_id
    )
    .fetch_all(exec)
    .await?
    .into_iter()
    .map(RunOutput::try_from)
    .collect()
}

/// Remove every hook output of one phase, for example before re-uploading a
/// phase with fewer hooks. Returns the number of rows removed.
pub async fn delete_phase(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
    phase: Phase,
) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM run_outputs WHERE org_id = $1 AND run_id = $2 AND phase = $3",
        org_id,
        run_id,
        phase.as_str()
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::is_foreign_key_violation;
    use crate::db::runs::{self, NewRun};
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::vaults;
    use serde_json::json;

    fn output<'a>(
        phase: Phase,
        hook_index: i32,
        hook_id: &'a str,
        value: &'a JsonValue,
    ) -> NewRunOutput<'a> {
        NewRunOutput {
            phase,
            hook_index,
            hook_id,
            output: value,
            config: None,
            success: true,
            error: None,
            failure_reason: None,
        }
    }

    #[tokio::test]
    async fn upsert_is_idempotent_and_keeps_array_order() {
        let pool = test_pool().await;
        let (_, org) = user_with_org(&pool, "alice").await;
        let vault = vaults::create(&pool, org.id, "sim").await.unwrap();
        runs::create(
            &pool,
            org.id,
            &NewRun::minimal("01RUN", vault.id, Utc::now()),
        )
        .await
        .unwrap();

        let git = json!({ "sha": "abc" });
        let env1 = json!({ "value": "a" });
        let env2 = json!({ "value": "b" });
        let post = json!({ "files": 3 });
        // Two instances of the same hook id in one phase are distinct rows.
        upsert(
            &pool,
            org.id,
            "01RUN",
            &output(Phase::Post, 0, "capture-file", &post),
        )
        .await
        .unwrap();
        upsert(
            &pool,
            org.id,
            "01RUN",
            &output(Phase::Pre, 2, "capture-env", &env2),
        )
        .await
        .unwrap();
        upsert(
            &pool,
            org.id,
            "01RUN",
            &output(Phase::Pre, 1, "capture-env", &env1),
        )
        .await
        .unwrap();
        let first = upsert(
            &pool,
            org.id,
            "01RUN",
            &output(Phase::Pre, 0, "capture-git-repo", &git),
        )
        .await
        .unwrap();

        let listed = list(&pool, org.id, "01RUN").await.unwrap();
        let summary: Vec<(Phase, i32, &str)> = listed
            .iter()
            .map(|o| (o.phase, o.hook_index, o.hook_id.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (Phase::Pre, 0, "capture-git-repo"),
                (Phase::Pre, 1, "capture-env"),
                (Phase::Pre, 2, "capture-env"),
                (Phase::Post, 0, "capture-file"),
            ]
        );
        assert_eq!(listed[0], first);

        // Re-uploading replaces the row in place.
        let failed = NewRunOutput {
            phase: Phase::Pre,
            hook_index: 0,
            hook_id: "capture-git-repo",
            output: &json!({}),
            config: Some(&json!({ "path": "." })),
            success: false,
            error: Some("not a git repository"),
            failure_reason: Some("not_a_repository"),
        };
        let replaced = upsert(&pool, org.id, "01RUN", &failed).await.unwrap();
        assert_eq!(replaced.id, first.id);
        assert!(!replaced.success);
        assert_eq!(replaced.error.as_deref(), Some("not a git repository"));
        assert_eq!(replaced.failure_reason.as_deref(), Some("not_a_repository"));
        assert_eq!(replaced.config, Some(json!({ "path": "." })));
        assert_eq!(list(&pool, org.id, "01RUN").await.unwrap().len(), 4);

        assert_eq!(
            delete_phase(&pool, org.id, "01RUN", Phase::Pre)
                .await
                .unwrap(),
            3
        );
        assert_eq!(list(&pool, org.id, "01RUN").await.unwrap().len(), 1);

        // Deleting the run removes the rest.
        runs::delete(&pool, org.id, "01RUN").await.unwrap();
        assert_eq!(list(&pool, org.id, "01RUN").await.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn outputs_need_a_run_in_the_same_org() {
        let pool = test_pool().await;
        let (_, org) = user_with_org(&pool, "alice").await;
        let (_, other) = user_with_org(&pool, "bob").await;
        let vault = vaults::create(&pool, org.id, "sim").await.unwrap();
        runs::create(
            &pool,
            org.id,
            &NewRun::minimal("01RUN", vault.id, Utc::now()),
        )
        .await
        .unwrap();
        let value = json!({});
        let err = upsert(
            &pool,
            other.id,
            "01RUN",
            &output(Phase::Pre, 0, "x", &value),
        )
        .await
        .unwrap_err();
        assert!(is_foreign_key_violation(&err), "{err}");
        assert_eq!(list(&pool, other.id, "01RUN").await.unwrap(), vec![]);
    }
}
