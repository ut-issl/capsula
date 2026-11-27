//! Audit log: who did what in which organization. Entries outlive the org
//! and the user they refer to.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value as JsonValue;
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    /// `None` for account-level actions, or once the org row is gone.
    pub org_id: Option<Uuid>,
    /// `None` for system actions, or once the account is gone.
    pub actor_user_id: Option<Uuid>,
    /// A short verb such as `run.push`, `member.remove`, `invite.create`.
    pub action: String,
    /// What the action was applied to, such as a run id or a user slug.
    pub target: Option<String>,
    /// Anything needed to read the entry later, such as the before/after
    /// of a role change.
    pub detail: Option<JsonValue>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewAuditEntry<'a> {
    pub org_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
    pub action: &'a str,
    pub target: Option<&'a str>,
    pub detail: Option<&'a JsonValue>,
}

pub async fn record(
    exec: impl PgExecutor<'_>,
    new: &NewAuditEntry<'_>,
) -> sqlx::Result<AuditEntry> {
    sqlx::query_as!(
        AuditEntry,
        r#"
        INSERT INTO audit_logs (org_id, actor_user_id, action, target, detail)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, org_id, actor_user_id, action, target, detail, at
        "#,
        new.org_id,
        new.actor_user_id,
        new.action,
        new.target,
        new.detail
    )
    .fetch_one(exec)
    .await
}

/// A page of an organization's entries, newest first.
pub async fn list(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<AuditEntry>> {
    sqlx::query_as!(
        AuditEntry,
        r#"
        SELECT id, org_id, actor_user_id, action, target, detail, at
        FROM audit_logs
        WHERE org_id = $1
        ORDER BY at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
        org_id,
        limit,
        offset
    )
    .fetch_all(exec)
    .await
}

pub async fn count(exec: impl PgExecutor<'_>, org_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM audit_logs WHERE org_id = $1"#,
        org_id
    )
    .fetch_one(exec)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::orgs;
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::users;
    use serde_json::json;

    #[tokio::test]
    async fn record_list_count_and_survive_deletions() {
        let pool = test_pool().await;
        let (alice, org) = user_with_org(&pool, "alice").await;
        let (_, other) = user_with_org(&pool, "bob").await;

        let detail = json!({ "from": "member", "to": "owner" });
        let first = record(
            &pool,
            &NewAuditEntry {
                org_id: Some(org.id),
                actor_user_id: Some(alice.id),
                action: "member.role",
                target: Some("carol"),
                detail: Some(&detail),
            },
        )
        .await
        .unwrap();
        assert_eq!(first.detail, Some(detail));
        for i in 0..3 {
            record(
                &pool,
                &NewAuditEntry {
                    org_id: Some(org.id),
                    actor_user_id: Some(alice.id),
                    action: "run.push",
                    target: Some(&format!("01RUN{i}")),
                    detail: None,
                },
            )
            .await
            .unwrap();
        }
        record(
            &pool,
            &NewAuditEntry {
                org_id: Some(other.id),
                actor_user_id: None,
                action: "org.create",
                target: None,
                detail: None,
            },
        )
        .await
        .unwrap();
        record(
            &pool,
            &NewAuditEntry {
                org_id: None,
                actor_user_id: Some(alice.id),
                action: "login",
                target: None,
                detail: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(count(&pool, org.id).await.unwrap(), 4);
        let page = list(&pool, org.id, 2, 0).await.unwrap();
        let targets: Vec<Option<&str>> = page.iter().map(|e| e.target.as_deref()).collect();
        assert_eq!(targets, [Some("01RUN2"), Some("01RUN1")]);
        let rest = list(&pool, org.id, 10, 2).await.unwrap();
        assert_eq!(rest.len(), 2);
        assert_eq!(rest[1], first);
        assert_eq!(count(&pool, other.id).await.unwrap(), 1);

        // Deleting the actor and the org keeps the entries.
        users::delete(&pool, alice.id).await.unwrap();
        assert_eq!(count(&pool, org.id).await.unwrap(), 4);
        assert!(
            list(&pool, org.id, 10, 0)
                .await
                .unwrap()
                .iter()
                .all(|e| e.actor_user_id.is_none())
        );
        orgs::hard_delete(&pool, org.id).await.unwrap();
        assert_eq!(count(&pool, org.id).await.unwrap(), 0);
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(total, 6);
    }
}
