//! SSH public keys registered for `capsula login`. Keys belong to users; a
//! fingerprint can be registered by at most one user.

use super::decode_enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    /// Pasted by the user.
    Manual,
    /// Imported from `github.com/<user>.keys`.
    Github,
}

impl KeySource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Github => "github",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "github" => Some(Self::Github),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshKey {
    pub id: Uuid,
    pub user_id: Uuid,
    /// SHA-256 fingerprint in OpenSSH form (`SHA256:...`).
    pub fingerprint: String,
    /// The key in OpenSSH authorized-keys form.
    pub public_key: String,
    pub title: String,
    pub source: KeySource,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewSshKey<'a> {
    pub user_id: Uuid,
    pub fingerprint: &'a str,
    pub public_key: &'a str,
    pub title: &'a str,
    pub source: KeySource,
}

struct Row {
    id: Uuid,
    user_id: Uuid,
    fingerprint: String,
    public_key: String,
    title: String,
    source: String,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<Row> for SshKey {
    type Error = sqlx::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            fingerprint: row.fingerprint,
            public_key: row.public_key,
            title: row.title,
            source: decode_enum("source", &row.source, KeySource::parse)?,
            last_used_at: row.last_used_at,
            created_at: row.created_at,
        })
    }
}

/// Register a key. Fails with a unique violation when the fingerprint is
/// already registered, by this or another user.
pub async fn create(exec: impl PgExecutor<'_>, new: &NewSshKey<'_>) -> sqlx::Result<SshKey> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        Row,
        r#"
        INSERT INTO ssh_keys (id, user_id, fingerprint, public_key, title, source)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, fingerprint, public_key, title, source, last_used_at, created_at
        "#,
        id,
        new.user_id,
        new.fingerprint,
        new.public_key,
        new.title,
        new.source.as_str()
    )
    .fetch_one(exec)
    .await?
    .try_into()
}

pub async fn get_by_fingerprint(
    exec: impl PgExecutor<'_>,
    fingerprint: &str,
) -> sqlx::Result<Option<SshKey>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, user_id, fingerprint, public_key, title, source, last_used_at, created_at
        FROM ssh_keys
        WHERE fingerprint = $1
        "#,
        fingerprint
    )
    .fetch_optional(exec)
    .await?
    .map(SshKey::try_from)
    .transpose()
}

/// A user's keys, oldest first.
pub async fn list_for_user(exec: impl PgExecutor<'_>, user_id: Uuid) -> sqlx::Result<Vec<SshKey>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, user_id, fingerprint, public_key, title, source, last_used_at, created_at
        FROM ssh_keys
        WHERE user_id = $1
        ORDER BY created_at, id
        "#,
        user_id
    )
    .fetch_all(exec)
    .await?
    .into_iter()
    .map(SshKey::try_from)
    .collect()
}

/// Record that the key was used to log in. Returns whether the key exists.
pub async fn touch(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    used_at: DateTime<Utc>,
) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "UPDATE ssh_keys SET last_used_at = $2 WHERE id = $1",
        id,
        used_at
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Delete one of the user's keys. The `login` tokens issued through it are
/// removed by the cascade. Returns whether a key of this user was deleted;
/// another user's key is never touched.
pub async fn delete(exec: impl PgExecutor<'_>, user_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM ssh_keys WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::is_unique_violation;
    use crate::db::test_support::{test_pool, user_with_org};
    use chrono::SubsecRound;

    fn key<'a>(user_id: Uuid, fingerprint: &'a str, title: &'a str) -> NewSshKey<'a> {
        NewSshKey {
            user_id,
            fingerprint,
            public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI... comment",
            title,
            source: KeySource::Manual,
        }
    }

    #[tokio::test]
    async fn register_lookup_touch_delete() {
        let pool = test_pool().await;
        let (alice, _) = user_with_org(&pool, "alice").await;
        let (bob, _) = user_with_org(&pool, "bob").await;

        let laptop = create(&pool, &key(alice.id, "SHA256:aaa", "laptop"))
            .await
            .unwrap();
        let desktop = create(
            &pool,
            &NewSshKey {
                source: KeySource::Github,
                ..key(alice.id, "SHA256:bbb", "desktop")
            },
        )
        .await
        .unwrap();
        assert_eq!(desktop.source, KeySource::Github);

        // A fingerprint is registered at most once, across users.
        let err = create(&pool, &key(bob.id, "SHA256:aaa", "stolen"))
            .await
            .unwrap_err();
        assert!(is_unique_violation(&err), "{err}");

        assert_eq!(
            get_by_fingerprint(&pool, "SHA256:aaa").await.unwrap(),
            Some(laptop.clone())
        );
        assert_eq!(get_by_fingerprint(&pool, "SHA256:zzz").await.unwrap(), None);

        let now = Utc::now().trunc_subsecs(6);
        assert!(touch(&pool, laptop.id, now).await.unwrap());
        assert!(!touch(&pool, Uuid::new_v4(), now).await.unwrap());

        let listed = list_for_user(&pool, alice.id).await.unwrap();
        let titles: Vec<&str> = listed.iter().map(|k| k.title.as_str()).collect();
        assert_eq!(titles, ["laptop", "desktop"]);
        assert_eq!(listed[0].last_used_at, Some(now));
        assert_eq!(list_for_user(&pool, bob.id).await.unwrap(), vec![]);

        assert!(!delete(&pool, bob.id, laptop.id).await.unwrap());
        assert!(delete(&pool, alice.id, laptop.id).await.unwrap());
        assert!(!delete(&pool, alice.id, laptop.id).await.unwrap());
        assert_eq!(get_by_fingerprint(&pool, "SHA256:aaa").await.unwrap(), None);
        // The fingerprint is free again.
        create(&pool, &key(bob.id, "SHA256:aaa", "mine now"))
            .await
            .unwrap();
    }
}
