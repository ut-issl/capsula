//! Captured files of a run. Rows hold the metadata; the content lives in
//! object storage under `storage_key` and is shared by every run of the
//! organization with the same content.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturedFile {
    pub id: i64,
    pub org_id: Uuid,
    pub run_id: String,
    /// Path relative to the run directory, `/`-separated.
    pub path: String,
    pub size: i64,
    /// Lowercase hex SHA-256 of the content.
    pub hash: String,
    /// Key of the content in object storage.
    pub storage_key: String,
    pub content_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewCapturedFile<'a> {
    pub path: &'a str,
    pub size: i64,
    pub hash: &'a str,
    pub storage_key: &'a str,
    pub content_type: Option<&'a str>,
}

/// Insert a captured file, replacing the row with the same path so that
/// pushing a run twice does not duplicate anything. Fails with a foreign-key
/// violation when the run does not exist in the org.
pub async fn upsert(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
    new: &NewCapturedFile<'_>,
) -> sqlx::Result<CapturedFile> {
    sqlx::query_as!(
        CapturedFile,
        r#"
        INSERT INTO captured_files (org_id, run_id, path, size, hash, storage_key, content_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (org_id, run_id, path) DO UPDATE
        SET size = EXCLUDED.size,
            hash = EXCLUDED.hash,
            storage_key = EXCLUDED.storage_key,
            content_type = EXCLUDED.content_type
        RETURNING id, org_id, run_id, path, size, hash, storage_key, content_type, created_at
        "#,
        org_id,
        run_id,
        new.path,
        new.size,
        new.hash,
        new.storage_key,
        new.content_type
    )
    .fetch_one(exec)
    .await
}

/// The files of a run, ordered by path.
pub async fn list(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
) -> sqlx::Result<Vec<CapturedFile>> {
    sqlx::query_as!(
        CapturedFile,
        r#"
        SELECT id, org_id, run_id, path, size, hash, storage_key, content_type, created_at
        FROM captured_files
        WHERE org_id = $1 AND run_id = $2
        ORDER BY path
        "#,
        org_id,
        run_id
    )
    .fetch_all(exec)
    .await
}

pub async fn get(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
    path: &str,
) -> sqlx::Result<Option<CapturedFile>> {
    sqlx::query_as!(
        CapturedFile,
        r#"
        SELECT id, org_id, run_id, path, size, hash, storage_key, content_type, created_at
        FROM captured_files
        WHERE org_id = $1 AND run_id = $2 AND path = $3
        "#,
        org_id,
        run_id,
        path
    )
    .fetch_optional(exec)
    .await
}

/// The distinct storage keys referenced by a run, for cleaning up objects
/// after the run is deleted.
pub async fn storage_keys(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    run_id: &str,
) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!(
        r#"
        SELECT DISTINCT storage_key
        FROM captured_files
        WHERE org_id = $1 AND run_id = $2
        ORDER BY storage_key
        "#,
        org_id,
        run_id
    )
    .fetch_all(exec)
    .await
}

/// Whether any run of the organization still references `storage_key`.
pub async fn is_referenced(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    storage_key: &str,
) -> sqlx::Result<bool> {
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM captured_files WHERE org_id = $1 AND storage_key = $2
        ) AS "exists!"
        "#,
        org_id,
        storage_key
    )
    .fetch_one(exec)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::is_foreign_key_violation;
    use crate::db::runs::{self, NewRun};
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::vaults;

    fn file<'a>(path: &'a str, hash: &'a str, key: &'a str) -> NewCapturedFile<'a> {
        NewCapturedFile {
            path,
            size: 3,
            hash,
            storage_key: key,
            content_type: Some("text/plain"),
        }
    }

    #[tokio::test]
    async fn upsert_list_get_and_reference_counting() {
        let pool = test_pool().await;
        let (_, org) = user_with_org(&pool, "alice").await;
        let (_, other) = user_with_org(&pool, "bob").await;
        let vault = vaults::create(&pool, org.id, "sim").await.unwrap();
        for id in ["01A", "01B"] {
            runs::create(&pool, org.id, &NewRun::minimal(id, vault.id, Utc::now()))
                .await
                .unwrap();
        }
        let shared_key = format!("{}/sha256/aa/bb/aabb", org.id);
        let only_a_key = format!("{}/sha256/cc/dd/ccdd", org.id);

        let a_out = upsert(
            &pool,
            org.id,
            "01A",
            &file("out/result.txt", "aabb", &shared_key),
        )
        .await
        .unwrap();
        upsert(&pool, org.id, "01A", &file("log.txt", "ccdd", &only_a_key))
            .await
            .unwrap();
        upsert(
            &pool,
            org.id,
            "01B",
            &file("result.txt", "aabb", &shared_key),
        )
        .await
        .unwrap();

        let paths: Vec<String> = list(&pool, org.id, "01A")
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(paths, ["log.txt", "out/result.txt"]);
        assert_eq!(
            get(&pool, org.id, "01A", "out/result.txt").await.unwrap(),
            Some(a_out.clone())
        );
        assert_eq!(
            get(&pool, other.id, "01A", "out/result.txt").await.unwrap(),
            None
        );

        // Re-uploading the same path replaces the row.
        let replaced = upsert(
            &pool,
            org.id,
            "01A",
            &NewCapturedFile {
                path: "out/result.txt",
                size: 10,
                hash: "eeff",
                storage_key: &format!("{}/sha256/ee/ff/eeff", org.id),
                content_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(replaced.id, a_out.id);
        assert_eq!(replaced.size, 10);
        assert_eq!(replaced.content_type, None);
        assert_eq!(list(&pool, org.id, "01A").await.unwrap().len(), 2);

        // Reference counting across the runs of the org.
        let mut keys = storage_keys(&pool, org.id, "01A").await.unwrap();
        keys.sort();
        assert_eq!(keys.len(), 2);
        assert!(is_referenced(&pool, org.id, &shared_key).await.unwrap());
        assert!(is_referenced(&pool, org.id, &only_a_key).await.unwrap());
        assert!(!is_referenced(&pool, other.id, &shared_key).await.unwrap());

        runs::delete(&pool, org.id, "01A").await.unwrap();
        assert_eq!(list(&pool, org.id, "01A").await.unwrap(), vec![]);
        assert!(is_referenced(&pool, org.id, &shared_key).await.unwrap());
        assert!(!is_referenced(&pool, org.id, &only_a_key).await.unwrap());
    }

    #[tokio::test]
    async fn files_need_a_run_in_the_same_org() {
        let pool = test_pool().await;
        let (_, org) = user_with_org(&pool, "alice").await;
        let (_, other) = user_with_org(&pool, "bob").await;
        let vault = vaults::create(&pool, org.id, "sim").await.unwrap();
        runs::create(&pool, org.id, &NewRun::minimal("01A", vault.id, Utc::now()))
            .await
            .unwrap();
        let err = upsert(&pool, other.id, "01A", &file("x", "h", "k"))
            .await
            .unwrap_err();
        assert!(is_foreign_key_violation(&err), "{err}");
    }
}
