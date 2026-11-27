//! Who belongs to an organization, and as what. Access to an organization is
//! decided only by this table.

use super::decode_enum;
use super::users::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Member,
}

impl Role {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Member => "member",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(Self::Owner),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Membership {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

/// A member of an organization with their account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Member {
    pub user: User,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

/// Add a user to an organization. Fails with a unique violation when the user
/// is already a member and with a foreign-key violation when the org or user
/// does not exist.
pub async fn add(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    user_id: Uuid,
    role: Role,
) -> sqlx::Result<Membership> {
    let row = sqlx::query!(
        r#"
        INSERT INTO memberships (org_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING org_id, user_id, role, created_at
        "#,
        org_id,
        user_id,
        role.as_str()
    )
    .fetch_one(exec)
    .await?;
    Ok(Membership {
        org_id: row.org_id,
        user_id: row.user_id,
        role: decode_enum("role", &row.role, Role::parse)?,
        created_at: row.created_at,
    })
}

pub async fn get(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<Option<Membership>> {
    let row = sqlx::query!(
        r#"
        SELECT org_id, user_id, role, created_at
        FROM memberships
        WHERE org_id = $1 AND user_id = $2
        "#,
        org_id,
        user_id
    )
    .fetch_optional(exec)
    .await?;
    row.map(|row| {
        Ok(Membership {
            org_id: row.org_id,
            user_id: row.user_id,
            role: decode_enum("role", &row.role, Role::parse)?,
            created_at: row.created_at,
        })
    })
    .transpose()
}

/// The members of an organization: owners first, then by user slug.
pub async fn list(exec: impl PgExecutor<'_>, org_id: Uuid) -> sqlx::Result<Vec<Member>> {
    let rows = sqlx::query!(
        r#"
        SELECT u.id, u.slug, u.display_name, u.primary_email, u.accepted_terms_at,
               u.created_at, m.role, m.created_at AS joined_at
        FROM memberships m
        JOIN users u ON u.id = m.user_id
        WHERE m.org_id = $1
        ORDER BY (m.role = 'owner') DESC, u.slug
        "#,
        org_id
    )
    .fetch_all(exec)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(Member {
                user: User {
                    id: row.id,
                    slug: row.slug,
                    display_name: row.display_name,
                    primary_email: row.primary_email,
                    accepted_terms_at: row.accepted_terms_at,
                    created_at: row.created_at,
                },
                role: decode_enum("role", &row.role, Role::parse)?,
                joined_at: row.joined_at,
            })
        })
        .collect()
}

/// Change a member's role. Returns whether the membership exists.
pub async fn set_role(
    exec: impl PgExecutor<'_>,
    org_id: Uuid,
    user_id: Uuid,
    role: Role,
) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE memberships
        SET role = $3
        WHERE org_id = $1 AND user_id = $2
        "#,
        org_id,
        user_id,
        role.as_str()
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Remove a member. Their runs stay in the organization. Returns whether the
/// membership existed.
pub async fn remove(exec: impl PgExecutor<'_>, org_id: Uuid, user_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM memberships WHERE org_id = $1 AND user_id = $2",
        org_id,
        user_id
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// How many owners an organization has. Callers refuse to remove or demote
/// the last one.
pub async fn count_owners(exec: impl PgExecutor<'_>, org_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!"
        FROM memberships
        WHERE org_id = $1 AND role = 'owner'
        "#,
        org_id
    )
    .fetch_one(exec)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::orgs::{self, NewOrg, OrgKind};
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::{is_foreign_key_violation, is_unique_violation};

    #[tokio::test]
    async fn add_list_change_remove() {
        let pool = test_pool().await;
        let (alice, _) = user_with_org(&pool, "alice").await;
        let (bob, _) = user_with_org(&pool, "bob").await;
        let (carol, _) = user_with_org(&pool, "carol").await;
        let lab = orgs::create(
            &pool,
            &NewOrg {
                slug: "xx-lab",
                name: "XX Lab",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap();

        let m = add(&pool, lab.id, carol.id, Role::Owner).await.unwrap();
        assert_eq!(
            (m.org_id, m.user_id, m.role),
            (lab.id, carol.id, Role::Owner)
        );
        add(&pool, lab.id, alice.id, Role::Member).await.unwrap();
        add(&pool, lab.id, bob.id, Role::Member).await.unwrap();

        let err = add(&pool, lab.id, alice.id, Role::Owner).await.unwrap_err();
        assert!(is_unique_violation(&err), "{err}");
        let err = add(&pool, Uuid::new_v4(), alice.id, Role::Member)
            .await
            .unwrap_err();
        assert!(is_foreign_key_violation(&err), "{err}");

        let members = list(&pool, lab.id).await.unwrap();
        let summary: Vec<(&str, Role)> = members
            .iter()
            .map(|m| (m.user.slug.as_str(), m.role))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("carol", Role::Owner),
                ("alice", Role::Member),
                ("bob", Role::Member)
            ]
        );
        assert_eq!(members[1].user, alice);
        assert_eq!(count_owners(&pool, lab.id).await.unwrap(), 1);

        assert_eq!(
            get(&pool, lab.id, alice.id).await.unwrap().map(|m| m.role),
            Some(Role::Member)
        );
        assert!(
            set_role(&pool, lab.id, alice.id, Role::Owner)
                .await
                .unwrap()
        );
        assert_eq!(count_owners(&pool, lab.id).await.unwrap(), 2);
        assert!(
            !set_role(&pool, lab.id, Uuid::new_v4(), Role::Owner)
                .await
                .unwrap()
        );

        assert!(remove(&pool, lab.id, bob.id).await.unwrap());
        assert!(!remove(&pool, lab.id, bob.id).await.unwrap());
        assert_eq!(get(&pool, lab.id, bob.id).await.unwrap(), None);
        // Bob's personal membership is untouched.
        assert!(get(&pool, bob.id, bob.id).await.unwrap().is_none()); // org id != user id
        assert_eq!(orgs::list_for_user(&pool, bob.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn deleting_a_user_or_org_cascades() {
        let pool = test_pool().await;
        let (alice, personal) = user_with_org(&pool, "alice").await;
        let lab = orgs::create(
            &pool,
            &NewOrg {
                slug: "xx-lab",
                name: "XX Lab",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap();
        add(&pool, lab.id, alice.id, Role::Owner).await.unwrap();

        assert!(orgs::hard_delete(&pool, lab.id).await.unwrap());
        assert_eq!(get(&pool, lab.id, alice.id).await.unwrap(), None);
        assert!(get(&pool, personal.id, alice.id).await.unwrap().is_some());

        assert!(crate::db::users::delete(&pool, alice.id).await.unwrap());
        assert_eq!(get(&pool, personal.id, alice.id).await.unwrap(), None);
        assert_eq!(count_owners(&pool, personal.id).await.unwrap(), 0);
    }
}
