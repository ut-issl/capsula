//! Invitation links to an organization. Only the hash of the code is stored.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

/// An invitation without its code hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Invitation {
    pub id: Uuid,
    pub org_id: Uuid,
    /// The owner who created it; `None` once that account is gone.
    pub created_by: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    /// `None` means unlimited.
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewInvitation<'a> {
    pub created_by: Option<Uuid>,
    /// SHA-256 of the invitation code, hex encoded.
    pub code_hash: &'a str,
    pub expires_at: DateTime<Utc>,
    pub max_uses: Option<i32>,
}

/// Create an invitation to `org_id`. Fails with a unique violation when the
/// code hash exists.
pub async fn create(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    new: &NewInvitation<'_>,
) -> sqlx::Result<Invitation> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        Invitation,
        r#"
        INSERT INTO invitations (id, org_id, created_by, code_hash, expires_at, max_uses)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, org_id, created_by, expires_at, max_uses, uses, revoked_at, created_at
        "#,
        id,
        org_id,
        new.created_by,
        new.code_hash,
        new.expires_at,
        new.max_uses
    )
    .fetch_one(exec)
    .await
}

/// The invitation with this code hash, if it can still be redeemed: not
/// revoked, not expired and with uses left. Not org-scoped because the code
/// is what tells the acceptance flow which org it is for.
pub async fn find_redeemable(
    exec: impl PgExecutor<'_>,
    code_hash: &str,
) -> sqlx::Result<Option<Invitation>> {
    sqlx::query_as!(
        Invitation,
        r#"
        SELECT id, org_id, created_by, expires_at, max_uses, uses, revoked_at, created_at
        FROM invitations
        WHERE code_hash = $1
          AND revoked_at IS NULL
          AND expires_at > NOW()
          AND (max_uses IS NULL OR uses < max_uses)
        "#,
        code_hash
    )
    .fetch_optional(exec)
    .await
}

/// Count one use of the invitation. Checks the limits again atomically, so
/// two concurrent acceptances cannot exceed `max_uses`. Returns whether the
/// use was counted.
pub async fn redeem(exec: impl PgExecutor<'_>, org_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE invitations
        SET uses = uses + 1
        WHERE id = $1 AND org_id = $2
          AND revoked_at IS NULL
          AND expires_at > NOW()
          AND (max_uses IS NULL OR uses < max_uses)
        "#,
        id,
        org_id
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// The invitations of an organization, newest first, including revoked and
/// expired ones.
pub async fn list(exec: impl PgExecutor<'_>, org_id: Uuid) -> sqlx::Result<Vec<Invitation>> {
    sqlx::query_as!(
        Invitation,
        r#"
        SELECT id, org_id, created_by, expires_at, max_uses, uses, revoked_at, created_at
        FROM invitations
        WHERE org_id = $1
        ORDER BY created_at DESC, id DESC
        "#,
        org_id
    )
    .fetch_all(exec)
    .await
}

/// Revoke an invitation. Returns whether a live invitation of this org was revoked.
pub async fn revoke(exec: impl PgExecutor<'_>, org_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE invitations
        SET revoked_at = NOW()
        WHERE id = $1 AND org_id = $2 AND revoked_at IS NULL
        "#,
        id,
        org_id
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::orgs::{self, NewOrg, OrgKind};
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::{is_unique_violation, users};
    use chrono::Duration;

    struct Fixture {
        prof: Uuid,
        lab: Uuid,
        other: Uuid,
        two_uses: Invitation,
        unlimited: Invitation,
        expired: Invitation,
    }

    async fn seed(pool: &sqlx::PgPool) -> Fixture {
        let (prof, _) = user_with_org(pool, "prof").await;
        let lab = orgs::create(
            pool,
            &NewOrg {
                slug: "xx-lab",
                name: "XX Lab",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap();
        let (_, other) = user_with_org(pool, "other").await;
        let in_a_week = Utc::now() + Duration::days(7);
        let invite = |code_hash, expires_at, max_uses| NewInvitation {
            created_by: Some(prof.id),
            code_hash,
            expires_at,
            max_uses,
        };
        let two_uses = create(pool, lab.id, &invite("hash-two", in_a_week, Some(2)))
            .await
            .unwrap();
        let unlimited = create(pool, lab.id, &invite("hash-unlimited", in_a_week, None))
            .await
            .unwrap();
        let expired = create(
            pool,
            lab.id,
            &NewInvitation {
                created_by: None,
                ..invite("hash-expired", Utc::now() - Duration::minutes(1), None)
            },
        )
        .await
        .unwrap();
        Fixture {
            prof: prof.id,
            lab: lab.id,
            other: other.id,
            two_uses,
            unlimited,
            expired,
        }
    }

    #[tokio::test]
    async fn create_and_redeem() {
        let pool = test_pool().await;
        let f = seed(&pool).await;
        assert_eq!((f.two_uses.uses, f.two_uses.max_uses), (0, Some(2)));
        assert_eq!(f.expired.created_by, None);

        let err = create(
            &pool,
            f.other,
            &NewInvitation {
                created_by: None,
                code_hash: "hash-two",
                expires_at: Utc::now() + Duration::days(1),
                max_uses: None,
            },
        )
        .await
        .unwrap_err();
        assert!(is_unique_violation(&err), "{err}");

        assert_eq!(
            find_redeemable(&pool, "hash-two").await.unwrap(),
            Some(f.two_uses.clone())
        );
        assert_eq!(find_redeemable(&pool, "hash-expired").await.unwrap(), None);
        assert_eq!(find_redeemable(&pool, "hash-nope").await.unwrap(), None);

        // Uses are counted atomically and capped.
        assert!(redeem(&pool, f.lab, f.two_uses.id).await.unwrap());
        assert!(redeem(&pool, f.lab, f.two_uses.id).await.unwrap());
        assert!(!redeem(&pool, f.lab, f.two_uses.id).await.unwrap());
        assert_eq!(find_redeemable(&pool, "hash-two").await.unwrap(), None);
        assert!(!redeem(&pool, f.lab, f.expired.id).await.unwrap());
        // The wrong org cannot redeem.
        assert!(!redeem(&pool, f.other, f.unlimited.id).await.unwrap());
        for _ in 0..3 {
            assert!(redeem(&pool, f.lab, f.unlimited.id).await.unwrap());
        }
        assert_eq!(
            list(&pool, f.lab)
                .await
                .unwrap()
                .iter()
                .map(|i| i.uses)
                .sum::<i32>(),
            5
        );
    }

    #[tokio::test]
    async fn list_revoke_and_outlive_the_creator() {
        let pool = test_pool().await;
        let f = seed(&pool).await;
        redeem(&pool, f.lab, f.unlimited.id).await.unwrap();

        let listed = list(&pool, f.lab).await.unwrap();
        let summary: Vec<(&str, i32)> = listed
            .iter()
            .map(|i| {
                (
                    match i.id {
                        id if id == f.two_uses.id => "two",
                        id if id == f.unlimited.id => "unlimited",
                        _ => "expired",
                    },
                    i.uses,
                )
            })
            .collect();
        assert_eq!(summary, [("expired", 0), ("unlimited", 1), ("two", 0)]);
        assert_eq!(list(&pool, f.other).await.unwrap(), vec![]);

        assert!(!revoke(&pool, f.other, f.unlimited.id).await.unwrap());
        assert!(revoke(&pool, f.lab, f.unlimited.id).await.unwrap());
        assert!(!revoke(&pool, f.lab, f.unlimited.id).await.unwrap());
        assert_eq!(
            find_redeemable(&pool, "hash-unlimited").await.unwrap(),
            None
        );
        assert!(!redeem(&pool, f.lab, f.unlimited.id).await.unwrap());

        // The creator's account can go away; the invitation stays.
        users::delete(&pool, f.prof).await.unwrap();
        let listed = list(&pool, f.lab).await.unwrap();
        assert!(listed.iter().all(|i| i.created_by.is_none()));
        assert_eq!(listed.len(), 3);
    }
}
