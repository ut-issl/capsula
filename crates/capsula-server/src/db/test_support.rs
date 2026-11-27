//! A PostgreSQL container shared by the tests of this binary, with one fresh
//! database per test.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use tokio::sync::OnceCell;
use uuid::Uuid;

struct Server {
    _container: ContainerAsync<Postgres>,
    /// Connection URL without a database name.
    base_url: String,
}

static SERVER: OnceCell<Server> = OnceCell::const_new();

async fn server() -> &'static Server {
    SERVER
        .get_or_init(|| async {
            let container = Postgres::default()
                .with_tag("18")
                .with_env_var("POSTGRES_USER", "capsula")
                .with_env_var("POSTGRES_PASSWORD", "capsula_dev")
                .with_env_var("POSTGRES_DB", "postgres")
                .start()
                .await
                .expect("failed to start PostgreSQL");
            let host = container.get_host().await.unwrap();
            let port = container.get_host_port_ipv4(5432).await.unwrap();
            Server {
                _container: container,
                base_url: format!("postgres://capsula:capsula_dev@{host}:{port}"),
            }
        })
        .await
}

/// A pool on a new, empty database with the migrations applied.
pub async fn test_pool() -> PgPool {
    let server = server().await;
    let db_name = format!("test_{}", Uuid::new_v4().simple());

    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&format!("{}/postgres", server.base_url))
        .await
        .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE \"{db_name}\""
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&format!("{}/{db_name}", server.base_url))
        .await
        .unwrap();
    crate::migrator().run(&pool).await.unwrap();
    pool
}

/// A user and a personal organization owned by them, for tests that need an org.
pub async fn user_with_org(
    pool: &PgPool,
    slug: &str,
) -> (super::users::User, super::orgs::Organization) {
    let user = super::users::create(
        pool,
        &super::users::NewUser {
            slug,
            display_name: slug,
            primary_email: Some(&format!("{slug}@example.com")),
        },
    )
    .await
    .unwrap();
    let org = super::orgs::create(
        pool,
        &super::orgs::NewOrg {
            slug,
            name: slug,
            kind: super::orgs::OrgKind::Personal,
        },
    )
    .await
    .unwrap();
    super::memberships::add(pool, org.id, user.id, super::memberships::Role::Owner)
        .await
        .unwrap();
    (user, org)
}
