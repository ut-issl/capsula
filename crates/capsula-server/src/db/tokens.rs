//! Bearer tokens (`api_tokens`). Tokens belong to users, not organizations;
//! only the SHA-256 hash of a token is stored. Generating and hashing tokens
//! is the authentication layer's job.

use super::decode_enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    /// Short-lived token issued by `capsula login` through an SSH key.
    Login,
    /// Long-lived token created in the web UI for CI and scripts.
    Manual,
}

impl TokenKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Manual => "manual",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "login" => Some(Self::Login),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

/// A token without its hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: TokenKind,
    /// The SSH key a `login` token was issued through.
    pub ssh_key_id: Option<Uuid>,
    pub name: String,
    /// Public, non-secret prefix shown in the UI to identify the token.
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewToken<'a> {
    pub user_id: Uuid,
    pub kind: TokenKind,
    pub ssh_key_id: Option<Uuid>,
    pub name: &'a str,
    pub prefix: &'a str,
    /// SHA-256 of the token, hex encoded.
    pub token_hash: &'a str,
    pub expires_at: Option<DateTime<Utc>>,
}

struct Row {
    id: Uuid,
    user_id: Uuid,
    kind: String,
    ssh_key_id: Option<Uuid>,
    name: String,
    prefix: String,
    last_used_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<Row> for ApiToken {
    type Error = sqlx::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            user_id: row.user_id,
            kind: decode_enum("kind", &row.kind, TokenKind::parse)?,
            ssh_key_id: row.ssh_key_id,
            name: row.name,
            prefix: row.prefix,
            last_used_at: row.last_used_at,
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
        })
    }
}

/// Store a token. Fails with a unique violation when the hash exists.
pub async fn create(exec: impl PgExecutor<'_>, new: &NewToken<'_>) -> sqlx::Result<ApiToken> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        Row,
        r#"
        INSERT INTO api_tokens (id, user_id, kind, ssh_key_id, name, prefix, token_hash, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, user_id, kind, ssh_key_id, name, prefix, last_used_at, expires_at,
                  revoked_at, created_at
        "#,
        id,
        new.user_id,
        new.kind.as_str(),
        new.ssh_key_id,
        new.name,
        new.prefix,
        new.token_hash,
        new.expires_at
    )
    .fetch_one(exec)
    .await?
    .try_into()
}

/// The token with this hash, if it exists, is not revoked and has not expired.
pub async fn find_active_by_hash(
    exec: impl PgExecutor<'_>,
    token_hash: &str,
) -> sqlx::Result<Option<ApiToken>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, user_id, kind, ssh_key_id, name, prefix, last_used_at, expires_at,
               revoked_at, created_at
        FROM api_tokens
        WHERE token_hash = $1
          AND revoked_at IS NULL
          AND (expires_at IS NULL OR expires_at > NOW())
        "#,
        token_hash
    )
    .fetch_optional(exec)
    .await?
    .map(ApiToken::try_from)
    .transpose()
}

/// Record a use of the token and, for tokens whose lifetime is extended on
/// use, push the expiry to `extend_to`. Returns whether the token exists.
pub async fn touch(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    used_at: DateTime<Utc>,
    extend_to: Option<DateTime<Utc>>,
) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE api_tokens
        SET last_used_at = $2,
            expires_at = COALESCE($3, expires_at)
        WHERE id = $1
        "#,
        id,
        used_at,
        extend_to
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// A user's tokens that are not revoked, newest first. Expired tokens are
/// included so the UI can show them.
pub async fn list_for_user(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<Vec<ApiToken>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, user_id, kind, ssh_key_id, name, prefix, last_used_at, expires_at,
               revoked_at, created_at
        FROM api_tokens
        WHERE user_id = $1 AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(exec)
    .await?
    .into_iter()
    .map(ApiToken::try_from)
    .collect()
}

/// Revoke one of the user's tokens. Returns whether a live token of this user
/// was revoked; another user's token is never touched.
pub async fn revoke(exec: impl PgExecutor<'_>, user_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE api_tokens
        SET revoked_at = NOW()
        WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
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
    use crate::db::ssh_keys::{self, KeySource, NewSshKey};
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::{is_foreign_key_violation, is_unique_violation};
    use chrono::{Duration, SubsecRound};

    fn manual<'a>(user_id: Uuid, name: &'a str, hash: &'a str) -> NewToken<'a> {
        NewToken {
            user_id,
            kind: TokenKind::Manual,
            ssh_key_id: None,
            name,
            prefix: "cap_abcd",
            token_hash: hash,
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn create_find_touch_list_revoke() {
        let pool = test_pool().await;
        let (alice, _) = user_with_org(&pool, "alice").await;
        let (bob, _) = user_with_org(&pool, "bob").await;

        let ci = create(&pool, &manual(alice.id, "ci", "hash-ci"))
            .await
            .unwrap();
        assert_eq!(ci.kind, TokenKind::Manual);
        assert!(ci.expires_at.is_none());
        let err = create(&pool, &manual(bob.id, "dup", "hash-ci"))
            .await
            .unwrap_err();
        assert!(is_unique_violation(&err), "{err}");
        let err = create(&pool, &manual(Uuid::new_v4(), "x", "hash-x"))
            .await
            .unwrap_err();
        assert!(is_foreign_key_violation(&err), "{err}");

        let now = Utc::now().trunc_subsecs(6);
        let expired = create(
            &pool,
            &NewToken {
                expires_at: Some(now - Duration::minutes(1)),
                ..manual(alice.id, "old", "hash-old")
            },
        )
        .await
        .unwrap();
        let fresh = create(
            &pool,
            &NewToken {
                expires_at: Some(now + Duration::days(30)),
                ..manual(alice.id, "laptop", "hash-fresh")
            },
        )
        .await
        .unwrap();

        assert_eq!(
            find_active_by_hash(&pool, "hash-ci").await.unwrap(),
            Some(ci.clone())
        );
        assert_eq!(find_active_by_hash(&pool, "hash-old").await.unwrap(), None);
        assert_eq!(find_active_by_hash(&pool, "hash-nope").await.unwrap(), None);
        assert!(
            find_active_by_hash(&pool, "hash-fresh")
                .await
                .unwrap()
                .is_some()
        );

        // Touch extends the expiry when asked and records the use.
        let later = now + Duration::days(60);
        assert!(touch(&pool, fresh.id, now, Some(later)).await.unwrap());
        assert!(touch(&pool, ci.id, now, None).await.unwrap());
        assert!(!touch(&pool, Uuid::new_v4(), now, None).await.unwrap());

        let listed = list_for_user(&pool, alice.id).await.unwrap();
        let names: Vec<&str> = listed.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["laptop", "old", "ci"]);
        let fresh_now = listed.iter().find(|t| t.id == fresh.id).unwrap();
        assert_eq!(fresh_now.expires_at, Some(later));
        assert_eq!(fresh_now.last_used_at, Some(now));
        let ci_now = listed.iter().find(|t| t.id == ci.id).unwrap();
        assert_eq!(ci_now.expires_at, None);
        assert_eq!(list_for_user(&pool, bob.id).await.unwrap(), vec![]);

        // Bob cannot revoke Alice's token; Alice can, once.
        assert!(!revoke(&pool, bob.id, ci.id).await.unwrap());
        assert!(revoke(&pool, alice.id, ci.id).await.unwrap());
        assert!(!revoke(&pool, alice.id, ci.id).await.unwrap());
        assert_eq!(find_active_by_hash(&pool, "hash-ci").await.unwrap(), None);
        assert_eq!(list_for_user(&pool, alice.id).await.unwrap().len(), 2);
        let _ = expired;
    }

    #[tokio::test]
    async fn login_tokens_die_with_their_key_and_their_user() {
        let pool = test_pool().await;
        let (alice, _) = user_with_org(&pool, "alice").await;
        let key = ssh_keys::create(
            &pool,
            &NewSshKey {
                user_id: alice.id,
                fingerprint: "SHA256:abc",
                public_key: "ssh-ed25519 AAAA alice@laptop",
                title: "laptop",
                source: KeySource::Manual,
            },
        )
        .await
        .unwrap();
        let login = create(
            &pool,
            &NewToken {
                user_id: alice.id,
                kind: TokenKind::Login,
                ssh_key_id: Some(key.id),
                name: "capsula login",
                prefix: "cap_login",
                token_hash: "hash-login",
                expires_at: Some(Utc::now() + Duration::days(30)),
            },
        )
        .await
        .unwrap();
        assert_eq!(login.kind, TokenKind::Login);
        create(&pool, &manual(alice.id, "ci", "hash-ci"))
            .await
            .unwrap();

        assert!(ssh_keys::delete(&pool, alice.id, key.id).await.unwrap());
        assert_eq!(
            find_active_by_hash(&pool, "hash-login").await.unwrap(),
            None
        );
        assert!(
            find_active_by_hash(&pool, "hash-ci")
                .await
                .unwrap()
                .is_some()
        );

        crate::db::users::delete(&pool, alice.id).await.unwrap();
        assert_eq!(find_active_by_hash(&pool, "hash-ci").await.unwrap(), None);
    }
}
