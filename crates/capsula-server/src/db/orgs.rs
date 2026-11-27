//! Organizations: a lab (`team`) or a user's personal space (`personal`).

use super::decode_enum;
use super::memberships::Role;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrgKind {
    Personal,
    Team,
}

impl OrgKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Team => "team",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "personal" => Some(Self::Personal),
            "team" => Some(Self::Team),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub kind: OrgKind,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewOrg<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub kind: OrgKind,
}

/// An organization together with the caller's role in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrgWithRole {
    pub org: Organization,
    pub role: Role,
}

struct Row {
    id: Uuid,
    slug: String,
    name: String,
    kind: String,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for Organization {
    type Error = sqlx::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            slug: row.slug,
            name: row.name,
            kind: decode_enum("kind", &row.kind, OrgKind::parse)?,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
        })
    }
}

/// Insert an organization. Fails with a unique violation when the slug is taken.
pub async fn create(exec: impl PgExecutor<'_>, new: &NewOrg<'_>) -> sqlx::Result<Organization> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        Row,
        r#"
        INSERT INTO organizations (id, slug, name, kind)
        VALUES ($1, $2, $3, $4)
        RETURNING id, slug, name, kind, created_at, deleted_at
        "#,
        id,
        new.slug,
        new.name,
        new.kind.as_str()
    )
    .fetch_one(exec)
    .await?
    .try_into()
}

/// Fetch an organization that has not been deleted.
pub async fn get(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<Organization>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, slug, name, kind, created_at, deleted_at
        FROM organizations
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        id
    )
    .fetch_optional(exec)
    .await?
    .map(Organization::try_from)
    .transpose()
}

/// Fetch an organization by slug, ignoring deleted ones.
pub async fn get_by_slug(
    exec: impl PgExecutor<'_>,
    slug: &str,
) -> sqlx::Result<Option<Organization>> {
    sqlx::query_as!(
        Row,
        r#"
        SELECT id, slug, name, kind, created_at, deleted_at
        FROM organizations
        WHERE slug = $1 AND deleted_at IS NULL
        "#,
        slug
    )
    .fetch_optional(exec)
    .await?
    .map(Organization::try_from)
    .transpose()
}

/// The organizations a user is a member of, with the user's role, ordered by slug.
pub async fn list_for_user(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<Vec<OrgWithRole>> {
    let rows = sqlx::query!(
        r#"
        SELECT o.id, o.slug, o.name, o.kind, o.created_at, o.deleted_at, m.role
        FROM organizations o
        JOIN memberships m ON m.org_id = o.id
        WHERE m.user_id = $1 AND o.deleted_at IS NULL
        ORDER BY o.slug
        "#,
        user_id
    )
    .fetch_all(exec)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(OrgWithRole {
                org: Organization {
                    id: row.id,
                    slug: row.slug,
                    name: row.name,
                    kind: decode_enum("kind", &row.kind, OrgKind::parse)?,
                    created_at: row.created_at,
                    deleted_at: row.deleted_at,
                },
                role: decode_enum("role", &row.role, Role::parse)?,
            })
        })
        .collect()
}

/// Change the display name. Returns whether a live organization was updated.
pub async fn rename(exec: impl PgExecutor<'_>, id: Uuid, name: &str) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE organizations
        SET name = $2
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        id,
        name
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Mark an organization as deleted. The slug stays reserved; the rows below
/// it are removed separately once their objects have been deleted from
/// storage. Returns whether a live organization was marked.
pub async fn soft_delete(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE organizations
        SET deleted_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        id
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Remove an organization and everything below it (memberships, invitations,
/// vaults, runs, hook outputs, captured-file rows). Returns whether it existed.
pub async fn hard_delete(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query!("DELETE FROM organizations WHERE id = $1", id)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::memberships;
    use crate::db::test_support::{test_pool, user_with_org};
    use crate::db::{is_unique_violation, users};

    #[tokio::test]
    async fn create_get_rename_delete() {
        let pool = test_pool().await;
        let org = create(
            &pool,
            &NewOrg {
                slug: "xx-lab",
                name: "XX Lab",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap();
        assert_eq!(org.kind, OrgKind::Team);
        assert!(org.deleted_at.is_none());
        assert_eq!(get(&pool, org.id).await.unwrap(), Some(org.clone()));
        assert_eq!(
            get_by_slug(&pool, "xx-lab").await.unwrap(),
            Some(org.clone())
        );

        assert!(rename(&pool, org.id, "XX Laboratory").await.unwrap());
        assert_eq!(
            get(&pool, org.id).await.unwrap().unwrap().name,
            "XX Laboratory"
        );

        assert!(soft_delete(&pool, org.id).await.unwrap());
        assert!(!soft_delete(&pool, org.id).await.unwrap());
        assert_eq!(get(&pool, org.id).await.unwrap(), None);
        assert_eq!(get_by_slug(&pool, "xx-lab").await.unwrap(), None);
        assert!(!rename(&pool, org.id, "x").await.unwrap());

        // The slug stays reserved while the row exists.
        let err = create(
            &pool,
            &NewOrg {
                slug: "xx-lab",
                name: "Other",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap_err();
        assert!(is_unique_violation(&err), "{err}");

        assert!(hard_delete(&pool, org.id).await.unwrap());
        assert!(!hard_delete(&pool, org.id).await.unwrap());
    }

    #[tokio::test]
    async fn list_for_user_returns_live_orgs_with_roles() {
        let pool = test_pool().await;
        let (alice, personal) = user_with_org(&pool, "alice").await;
        let (_bob, bobs) = user_with_org(&pool, "bob").await;
        let lab = create(
            &pool,
            &NewOrg {
                slug: "xx-lab",
                name: "XX Lab",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap();
        memberships::add(&pool, lab.id, alice.id, Role::Member)
            .await
            .unwrap();
        let gone = create(
            &pool,
            &NewOrg {
                slug: "old-lab",
                name: "Old",
                kind: OrgKind::Team,
            },
        )
        .await
        .unwrap();
        memberships::add(&pool, gone.id, alice.id, Role::Owner)
            .await
            .unwrap();
        soft_delete(&pool, gone.id).await.unwrap();

        let orgs = list_for_user(&pool, alice.id).await.unwrap();
        let summary: Vec<(&str, Role)> =
            orgs.iter().map(|o| (o.org.slug.as_str(), o.role)).collect();
        assert_eq!(
            summary,
            vec![("alice", Role::Owner), ("xx-lab", Role::Member)]
        );
        assert!(!orgs.iter().any(|o| o.org.id == bobs.id));
        assert_eq!(orgs[0].org, personal);

        // A user with no memberships gets an empty list.
        let carol = users::create(
            &pool,
            &users::NewUser {
                slug: "carol",
                display_name: "Carol",
                primary_email: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(list_for_user(&pool, carol.id).await.unwrap(), vec![]);
    }
}
