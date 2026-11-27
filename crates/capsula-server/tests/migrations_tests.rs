//! The server starts on an empty database and creates every table.
#![cfg(test)]

use capsula_server::{build_app, create_pool, migrator};
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ImageExt, runners::AsyncRunner},
};

/// Every table of the baseline schema.
const TABLES: &[&str] = &[
    "users",
    "identities",
    "organizations",
    "memberships",
    "invitations",
    "ssh_keys",
    "auth_challenges",
    "api_tokens",
    "sessions",
    "vaults",
    "runs",
    "run_outputs",
    "captured_files",
    "audit_logs",
];

#[tokio::test]
async fn migrations_create_every_table_and_the_server_starts() {
    let container = Postgres::default()
        .with_tag("18")
        .with_env_var("POSTGRES_USER", "capsula")
        .with_env_var("POSTGRES_PASSWORD", "capsula_dev")
        .with_env_var("POSTGRES_DB", "capsula")
        .start()
        .await
        .unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let database_url =
        format!("postgres://capsula:capsula_dev@{host}:{port}/capsula?sslmode=disable");

    let pool = create_pool(&database_url, 5).await.unwrap();
    migrator().run(&pool).await.unwrap();

    let mut tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
         AND table_name <> '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    tables.sort_unstable();
    let mut expected: Vec<String> = TABLES.iter().map(|t| (*t).to_owned()).collect();
    expected.sort_unstable();
    assert_eq!(tables, expected);

    // Running the migrator again is a no-op.
    migrator().run(&pool).await.unwrap();

    let app = build_app(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let body: serde_json::Value = reqwest::get(format!("http://{addr}/health"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "connected");

    server.abort();
}
