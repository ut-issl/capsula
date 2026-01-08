#![allow(clippy::unwrap_used, reason = "Test code")]
use capsula_server::{build_app, create_pool};
use serde_json::json;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};
use tokio::task::JoinHandle;

struct TestContext {
    base_url: String,
    _container: ContainerAsync<Postgres>,
    server: JoinHandle<()>,
}

impl TestContext {
    async fn new() -> Self {
        let container = Postgres::default()
            .with_tag("18")
            .with_env_var("POSTGRES_USER", "capsula")
            .with_env_var("POSTGRES_PASSWORD", "capsula_dev")
            .with_env_var("POSTGRES_DB", "capsula")
            .start()
            .await
            .unwrap();

        let host = container.get_host().await.unwrap();
        let host_port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url =
            format!("postgres://capsula:capsula_dev@{host}:{host_port}/capsula?sslmode=disable");

        let pool = create_pool(&database_url).await.unwrap();
        let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let migrator = sqlx::migrate::Migrator::new(migrations_path).await.unwrap();
        migrator.run(&pool).await.unwrap();

        let app = build_app(pool);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            base_url: format!("http://{addr}"),
            _container: container,
            server,
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        self.server.abort();
    }
}

#[tokio::test]
async fn test_health_check() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", ctx.base_url()))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "connected");
}

#[tokio::test]
async fn test_create_and_get_run() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_data = json!({
        "id": "01TEST123456789ABCDEFGHIJ",
        "name": "test-integration",
        "timestamp": "2026-01-08T10:20:00Z",
        "command": "cargo test",
        "vault": "test-vault",
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 1000,
        "stdout": "test output",
        "stderr": null
    });

    let response = client
        .post(format!("{}/api/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "created");
    assert_eq!(body["run"]["id"], "01TEST123456789ABCDEFGHIJ");

    let response = client
        .get(format!(
            "{}/api/runs/01TEST123456789ABCDEFGHIJ",
            ctx.base_url()
        ))
        .send()
        .await
        .expect("Failed to get run");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["run"]["id"], "01TEST123456789ABCDEFGHIJ");
    assert_eq!(body["run"]["vault"], "test-vault");
}

#[tokio::test]
async fn test_list_vaults() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/vaults", ctx.base_url()))
        .send()
        .await
        .expect("Failed to get vaults");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert!(body["vaults"].is_array());
}

#[tokio::test]
async fn test_get_vault_info() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_data = json!({
        "id": "01TESTVAULTINFO123456789A",
        "name": "test-vault-info",
        "timestamp": "2026-01-08T10:21:00Z",
        "command": "echo vault",
        "vault": "default",
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 100,
        "stdout": "vault test",
        "stderr": null
    });

    let response = client
        .post(format!("{}/api/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 200);

    let response = client
        .get(format!("{}/api/vaults/default", ctx.base_url()))
        .send()
        .await
        .expect("Failed to get vault");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["exists"], true);

    let response = client
        .get(format!(
            "{}/api/vaults/nonexistent-vault-xyz",
            ctx.base_url()
        ))
        .send()
        .await
        .expect("Failed to get vault");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["exists"], false);
}

#[tokio::test]
async fn test_list_runs_with_filters() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/api/runs", ctx.base_url()))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert!(body["runs"].is_array());

    let response = client
        .get(format!("{}/api/runs?vault=default", ctx.base_url()))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");

    if let Some(runs) = body["runs"].as_array() {
        for run in runs {
            assert_eq!(run["vault"], "default");
        }
    }
}

#[tokio::test]
async fn test_pagination() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/api/runs?limit=2", ctx.base_url()))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 0);

    let runs = body["runs"].as_array().expect("runs should be array");
    assert!(runs.len() <= 2);

    let response = client
        .get(format!("{}/api/runs?limit=1&offset=1", ctx.base_url()))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["limit"], 1);
    assert_eq!(body["offset"], 1);
}

#[tokio::test]
async fn test_multipart_upload() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new()
        .text("file1", "content of file 1")
        .text("file2", "content of file 2");

    let response = client
        .post(format!("{}/api/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("Failed to send multipart request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["files_processed"], 2);
    assert!(body["total_bytes"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_single_file_upload_with_storage() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_data = json!({
        "id": "01FILETEST123456789ABCDEF",
        "name": "test-file-upload",
        "timestamp": "2026-01-08T10:30:00Z",
        "command": "echo test",
        "vault": "test-vault",
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 100,
        "stdout": null,
        "stderr": null
    });

    let response = client
        .post(format!("{}/api/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 200);

    let file_content = b"This is test file content for upload";
    let form = reqwest::multipart::Form::new()
        .text("run_id", "01FILETEST123456789ABCDEF")
        .text("path", "test_file.txt")
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_content.to_vec())
                .file_name("test_file.txt")
                .mime_str("text/plain")
                .expect("Failed to set MIME type"),
        );

    let response = client
        .post(format!("{}/api/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload file");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["files_processed"], 1);
    assert_eq!(body["total_bytes"], file_content.len() as u64);
}

#[tokio::test]
async fn test_multiple_file_upload_with_storage() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_data = json!({
        "id": "01FILEMULTI123456789ABCDE",
        "name": "test-multi-file-upload",
        "timestamp": "2026-01-08T11:00:00Z",
        "command": "echo multi",
        "vault": "test-vault",
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 100,
        "stdout": null,
        "stderr": null
    });

    let response = client
        .post(format!("{}/api/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 200);

    let first_content = b"First file content";
    let second_content = b"Second file content";
    let form = reqwest::multipart::Form::new()
        .text("run_id", "01FILEMULTI123456789ABCDE")
        .text("path", "logs/first.txt")
        .part(
            "file",
            reqwest::multipart::Part::bytes(first_content.to_vec())
                .file_name("first.txt")
                .mime_str("text/plain")
                .expect("Failed to set MIME type"),
        )
        .text("path", "results/second.txt")
        .part(
            "file",
            reqwest::multipart::Part::bytes(second_content.to_vec())
                .file_name("second.txt")
                .mime_str("text/plain")
                .expect("Failed to set MIME type"),
        );

    let response = client
        .post(format!("{}/api/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload files");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["files_processed"], 2);
    assert_eq!(
        body["total_bytes"],
        (first_content.len() + second_content.len()) as u64
    );
}
