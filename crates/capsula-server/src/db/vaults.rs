//! Vaults: the projects of an organization. A vault is created on first push.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Vault {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// A vault with the number of runs it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VaultSummary {
    #[serde(flatten)]
    pub vault: Vault,
    pub run_count: i64,
}

/// Insert a vault. Fails with a unique violation when the name is taken in the org.
pub async fn create(exec: impl PgExecutor<'_>, org_id: Uuid, name: &str) -> sqlx::Result<Vault> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        Vault,
        r#"
        INSERT INTO vaults (id, org_id, name)
        VALUES ($1, $2, $3)
        RETURNING id, org_id, name, created_at
        "#,
        id,
        org_id,
        name
    )
    .fetch_one(exec)
    .await
}

/// The vault with this name, created if it does not exist yet.
pub async fn get_or_create(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    name: &str,
) -> sqlx::Result<Vault> {
    let id = Uuid::now_v7();
    // The no-op update makes RETURNING yield the existing row on conflict.
    sqlx::query_as!(
        Vault,
        r#"
        INSERT INTO vaults (id, org_id, name)
        VALUES ($1, $2, $3)
        ON CONFLICT (org_id, name) DO UPDATE SET name = EXCLUDED.name
        RETURNING id, org_id, name, created_at
        "#,
        id,
        org_id,
        name
    )
    .fetch_one(exec)
    .await
}

pub async fn get(exec: impl PgExecutor<'_>, org_id: Uuid, id: Uuid) -> sqlx::Result<Option<Vault>> {
    sqlx::query_as!(
        Vault,
        r#"
        SELECT id, org_id, name, created_at
        FROM vaults
        WHERE org_id = $1 AND id = $2
        "#,
        org_id,
        id
    )
    .fetch_optional(exec)
    .await
}

pub async fn get_by_name(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    name: &str,
) -> sqlx::Result<Option<Vault>> {
    sqlx::query_as!(
        Vault,
        r#"
        SELECT id, org_id, name, created_at
        FROM vaults
        WHERE org_id = $1 AND name = $2
        "#,
        org_id,
        name
    )
    .fetch_optional(exec)
    .await
}

/// The vaults of an organization with their run counts, ordered by name.
pub async fn list(exec: impl PgExecutor<'_>, org_id: Uuid) -> sqlx::Result<Vec<VaultSummary>> {
    let rows = sqlx::query!(
        r#"
        SELECT v.id, v.org_id, v.name, v.created_at, COUNT(r.id) AS "run_count!"
        FROM vaults v
        LEFT JOIN runs r ON r.org_id = v.org_id AND r.vault_id = v.id
        WHERE v.org_id = $1
        GROUP BY v.id
        ORDER BY v.name
        "#,
        org_id
    )
    .fetch_all(exec)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| VaultSummary {
            vault: Vault {
                id: row.id,
                org_id: row.org_id,
                name: row.name,
                created_at: row.created_at,
            },
            run_count: row.run_count,
        })
        .collect())
}

/// The run count of one vault, or `None` when the vault does not exist.
pub async fn summary(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    name: &str,
) -> sqlx::Result<Option<VaultSummary>> {
    let row = sqlx::query!(
        r#"
        SELECT v.id, v.org_id, v.name, v.created_at, COUNT(r.id) AS "run_count!"
        FROM vaults v
        LEFT JOIN runs r ON r.org_id = v.org_id AND r.vault_id = v.id
        WHERE v.org_id = $1 AND v.name = $2
        GROUP BY v.id
        "#,
        org_id,
        name
    )
    .fetch_optional(exec)
    .await?;
    Ok(row.map(|row| VaultSummary {
        vault: Vault {
            id: row.id,
            org_id: row.org_id,
            name: row.name,
            created_at: row.created_at,
        },
        run_count: row.run_count,
    }))
}

/// Delete a vault and its runs. Returns whether it existed.
pub async fn delete(exec: impl PgExecutor<'_>, org_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM vaults WHERE org_id = $1 AND id = $2",
        org_id,
        id
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::is_unique_violation;
    use crate::db::runs::{self, NewRun};
    use crate::db::test_support::{test_pool, user_with_org};

    #[tokio::test]
    async fn create_get_list_delete() {
        let pool = test_pool().await;
        let (_, org) = user_with_org(&pool, "alice").await;
        let (_, other) = user_with_org(&pool, "bob").await;

        let sim = create(&pool, org.id, "satellite-sim").await.unwrap();
        assert_eq!((sim.org_id, sim.name.as_str()), (org.id, "satellite-sim"));
        let err = create(&pool, org.id, "satellite-sim").await.unwrap_err();
        assert!(is_unique_violation(&err), "{err}");
        // The same name is free in another org.
        let bobs = create(&pool, other.id, "satellite-sim").await.unwrap();
        assert_ne!(bobs.id, sim.id);

        assert_eq!(
            get_or_create(&pool, org.id, "satellite-sim").await.unwrap(),
            sim
        );
        let analysis = get_or_create(&pool, org.id, "analysis").await.unwrap();
        assert_ne!(analysis.id, sim.id);

        assert_eq!(get(&pool, org.id, sim.id).await.unwrap(), Some(sim.clone()));
        assert_eq!(get(&pool, other.id, sim.id).await.unwrap(), None);
        assert_eq!(
            get_by_name(&pool, org.id, "satellite-sim").await.unwrap(),
            Some(sim.clone())
        );

        for id in ["01A", "01B"] {
            runs::create(&pool, org.id, &NewRun::minimal(id, sim.id, Utc::now()))
                .await
                .unwrap();
        }
        let listed = list(&pool, org.id).await.unwrap();
        let summary_list: Vec<(&str, i64)> = listed
            .iter()
            .map(|s| (s.vault.name.as_str(), s.run_count))
            .collect();
        assert_eq!(summary_list, vec![("analysis", 0), ("satellite-sim", 2)]);
        assert_eq!(
            summary(&pool, org.id, "satellite-sim")
                .await
                .unwrap()
                .map(|s| s.run_count),
            Some(2)
        );
        assert_eq!(summary(&pool, org.id, "nope").await.unwrap(), None);
        assert_eq!(list(&pool, other.id).await.unwrap().len(), 1);

        assert!(delete(&pool, org.id, sim.id).await.unwrap());
        assert!(!delete(&pool, org.id, sim.id).await.unwrap());
        assert_eq!(runs::get(&pool, org.id, "01A").await.unwrap(), None);
        // Deleting through the wrong org does nothing.
        assert!(!delete(&pool, other.id, analysis.id).await.unwrap());
    }
}
