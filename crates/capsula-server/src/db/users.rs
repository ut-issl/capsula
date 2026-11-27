//! Accounts. Users are not scoped to an organization.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct User {
    pub id: Uuid,
    /// Also the slug of the user's personal organization.
    pub slug: String,
    pub display_name: String,
    pub primary_email: Option<String>,
    pub accepted_terms_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewUser<'a> {
    pub slug: &'a str,
    pub display_name: &'a str,
    pub primary_email: Option<&'a str>,
}

/// Insert a user. Fails with a unique violation when the slug is taken.
pub async fn create(exec: impl PgExecutor<'_>, new: &NewUser<'_>) -> sqlx::Result<User> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, slug, display_name, primary_email)
        VALUES ($1, $2, $3, $4)
        RETURNING id, slug, display_name, primary_email, accepted_terms_at, created_at
        "#,
        id,
        new.slug,
        new.display_name,
        new.primary_email
    )
    .fetch_one(exec)
    .await
}

pub async fn get(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, slug, display_name, primary_email, accepted_terms_at, created_at
        FROM users
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(exec)
    .await
}

pub async fn get_by_slug(exec: impl PgExecutor<'_>, slug: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, slug, display_name, primary_email, accepted_terms_at, created_at
        FROM users
        WHERE slug = $1
        "#,
        slug
    )
    .fetch_optional(exec)
    .await
}

/// Update the display name and primary email. Returns whether the user exists.
pub async fn update_profile(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    display_name: &str,
    primary_email: Option<&str>,
) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE users
        SET display_name = $2, primary_email = $3
        WHERE id = $1
        "#,
        id,
        display_name,
        primary_email
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Record that the user accepted the terms at `at`. Returns whether the user exists.
pub async fn accept_terms(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    at: DateTime<Utc>,
) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE users
        SET accepted_terms_at = $2
        WHERE id = $1
        "#,
        id,
        at
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Delete a user. Memberships, identities, keys, tokens and sessions cascade;
/// runs they pushed keep existing with `pushed_by_user_id` set to NULL.
/// Returns whether the user existed.
pub async fn delete(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", id)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{is_unique_violation, test_support::test_pool};
    use chrono::SubsecRound;

    #[tokio::test]
    async fn create_get_update_delete() {
        let pool = test_pool().await;
        let new = NewUser {
            slug: "alice",
            display_name: "Alice",
            primary_email: Some("alice@example.com"),
        };
        let user = create(&pool, &new).await.unwrap();
        assert_eq!(user.slug, "alice");
        assert_eq!(user.display_name, "Alice");
        assert_eq!(user.primary_email.as_deref(), Some("alice@example.com"));
        assert!(user.accepted_terms_at.is_none());

        assert_eq!(get(&pool, user.id).await.unwrap(), Some(user.clone()));
        assert_eq!(
            get_by_slug(&pool, "alice").await.unwrap(),
            Some(user.clone())
        );
        assert_eq!(get_by_slug(&pool, "bob").await.unwrap(), None);

        assert!(
            update_profile(&pool, user.id, "Alice B.", None)
                .await
                .unwrap()
        );
        // PostgreSQL stores microseconds; truncate so the round trip compares equal.
        let at = Utc::now().trunc_subsecs(6);
        assert!(accept_terms(&pool, user.id, at).await.unwrap());
        let updated = get(&pool, user.id).await.unwrap().unwrap();
        assert_eq!(updated.display_name, "Alice B.");
        assert_eq!(updated.primary_email, None);
        assert_eq!(updated.accepted_terms_at, Some(at));

        assert!(delete(&pool, user.id).await.unwrap());
        assert!(!delete(&pool, user.id).await.unwrap());
        assert_eq!(get(&pool, user.id).await.unwrap(), None);
        assert!(!update_profile(&pool, user.id, "x", None).await.unwrap());
    }

    #[tokio::test]
    async fn slug_is_unique() {
        let pool = test_pool().await;
        let new = NewUser {
            slug: "alice",
            display_name: "Alice",
            primary_email: None,
        };
        create(&pool, &new).await.unwrap();
        let err = create(&pool, &new).await.unwrap_err();
        assert!(is_unique_violation(&err), "{err}");
    }
}
