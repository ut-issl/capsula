//! Integration tests for Capsula server API endpoints.
#![cfg(test)]

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

        let pool = create_pool(&database_url, 5).await.unwrap();
        let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let migrator = sqlx::migrate::Migrator::new(migrations_path).await.unwrap();
        migrator.run(&pool).await.unwrap();

        let storage_path = std::env::temp_dir().join("capsula-test-storage");
        // Use 100MB limit for tests (same as production default)
        let app = build_app(pool, storage_path, 104_857_600);
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
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");

    assert_eq!(response.status(), 201);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "created");
    assert_eq!(body["run"]["id"], "01TEST123456789ABCDEFGHIJ");

    let response = client
        .get(format!(
            "{}/api/v1/runs/01TEST123456789ABCDEFGHIJ",
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
        .get(format!("{}/api/v1/vaults", ctx.base_url()))
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
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 201);

    let response = client
        .get(format!("{}/api/v1/vaults/default", ctx.base_url()))
        .send()
        .await
        .expect("Failed to get vault");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["exists"], true);

    let response = client
        .get(format!(
            "{}/api/v1/vaults/nonexistent-vault-xyz",
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
        .get(format!("{}/api/v1/runs", ctx.base_url()))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert!(body["runs"].is_array());

    let response = client
        .get(format!("{}/api/v1/runs?vault=default", ctx.base_url()))
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
        .get(format!("{}/api/v1/runs?limit=2", ctx.base_url()))
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
        .get(format!("{}/api/v1/runs?limit=1&offset=1", ctx.base_url()))
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
        .post(format!("{}/api/v1/upload", ctx.base_url()))
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
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 201);

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
        .post(format!("{}/api/v1/upload", ctx.base_url()))
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
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 201);

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
        .post(format!("{}/api/v1/upload", ctx.base_url()))
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

#[tokio::test]
async fn test_file_download() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    // Create a run
    let run_data = json!({
        "id": "01FILEDOWNLOAD123456789AB",
        "name": "test-file-download",
        "timestamp": "2026-01-08T12:00:00Z",
        "command": "echo download",
        "vault": "test-vault",
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 100,
        "stdout": null,
        "stderr": null
    });

    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 201);

    // Upload a file
    let file_content = b"This is downloadable content\nLine 2\nLine 3";
    let form = reqwest::multipart::Form::new()
        .text("run_id", "01FILEDOWNLOAD123456789AB")
        .text("path", "output/result.txt")
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_content.to_vec())
                .file_name("result.txt")
                .mime_str("text/plain")
                .expect("Failed to set MIME type"),
        );

    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload file");
    assert_eq!(response.status(), 200);

    // Download the file
    let response = client
        .get(format!(
            "{}/api/v1/runs/01FILEDOWNLOAD123456789AB/files/output/result.txt",
            ctx.base_url()
        ))
        .send()
        .await
        .expect("Failed to download file");

    assert_eq!(response.status(), 200);

    // Check Content-Type header
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type header should be present")
        .to_str()
        .expect("Content-Type should be valid string");
    assert_eq!(content_type, "text/plain");

    // Check Content-Disposition header
    let content_disposition = response
        .headers()
        .get("content-disposition")
        .expect("Content-Disposition header should be present")
        .to_str()
        .expect("Content-Disposition should be valid string");
    assert!(content_disposition.contains("result.txt"));
    assert!(content_disposition.contains("inline"));

    // Check file content
    let downloaded_content = response
        .bytes()
        .await
        .expect("Failed to read response body");
    assert_eq!(downloaded_content.as_ref(), file_content);

    // Test 404 for non-existent file
    let response = client
        .get(format!(
            "{}/api/v1/runs/01FILEDOWNLOAD123456789AB/files/nonexistent.txt",
            ctx.base_url()
        ))
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(response.status(), 404);

    // Test 404 for non-existent run
    let response = client
        .get(format!(
            "{}/api/v1/runs/01NONEXISTENT123456789AB/files/result.txt",
            ctx.base_url()
        ))
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_hook_outputs_storage() {
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    // Create a run
    let run_data = json!({
        "id": "01HOOKTEST123456789ABCDE",
        "name": "test-hooks",
        "timestamp": "2026-01-08T13:00:00Z",
        "command": "cargo test",
        "vault": "test-vault",
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 1000,
        "stdout": null,
        "stderr": null
    });

    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 201);

    // Create hook outputs JSON
    let pre_run_hooks = json!([
        {
            "__meta": {
                "id": "git",
                "config": {"allow_dirty": false},
                "success": true,
                "error": null
            },
            "commit": "abc123",
            "branch": "main",
            "dirty": false
        },
        {
            "__meta": {
                "id": "env",
                "config": null,
                "success": true,
                "error": null
            },
            "PATH": "/usr/bin",
            "HOME": "/home/user"
        }
    ]);

    let post_run_hooks = json!([
        {
            "__meta": {
                "id": "file",
                "config": {"paths": ["output.txt"]},
                "success": true,
                "error": null
            },
            "files": ["output.txt"]
        }
    ]);

    // Upload with hook outputs
    let form = reqwest::multipart::Form::new()
        .text("run_id", "01HOOKTEST123456789ABCDE")
        .text("pre_run", pre_run_hooks.to_string())
        .text("post_run", post_run_hooks.to_string());

    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload hooks");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["pre_run_hooks"], 2);
    assert_eq!(body["post_run_hooks"], 1);

    // Verify we can retrieve the hooks via GET /api/v1/runs/:id
    let response = client
        .get(format!(
            "{}/api/v1/runs/01HOOKTEST123456789ABCDE",
            ctx.base_url()
        ))
        .send()
        .await
        .expect("Failed to get run");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");

    // Check pre-run hooks
    let pre_hooks = body["pre_run_hooks"]
        .as_array()
        .expect("pre_run_hooks should be array");
    assert_eq!(pre_hooks.len(), 2);
    assert_eq!(pre_hooks[0]["__meta"]["id"], "git");
    assert_eq!(pre_hooks[0]["__meta"]["success"], true);
    assert_eq!(pre_hooks[0]["commit"], "abc123");
    assert_eq!(pre_hooks[0]["branch"], "main");
    assert_eq!(pre_hooks[1]["__meta"]["id"], "env");
    assert_eq!(pre_hooks[1]["__meta"]["success"], true);

    // Check post-run hooks
    let post_hooks = body["post_run_hooks"]
        .as_array()
        .expect("post_run_hooks should be array");
    assert_eq!(post_hooks.len(), 1);
    assert_eq!(post_hooks[0]["__meta"]["id"], "file");
    assert_eq!(post_hooks[0]["__meta"]["success"], true);
}

#[tokio::test]
async fn multiple_hooks_with_distinct_configs_coexist() {
    // Regression for #1017: two capture-json hooks with the same hook_id
    // but distinct configs must both survive on the server. With the new
    // hook_index column they sit at array positions 0 and 1.
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_id = "01HOOKIDX00000000000000A";
    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&json!({
            "id": run_id,
            "name": "multi-hooks-distinct",
            "timestamp": "2026-06-13T10:00:00Z",
            "command": "test",
            "vault": "hook-index-vault",
            "project_root": "/tmp/test",
            "exit_code": 0,
            "duration_ms": 100,
            "stdout": null,
            "stderr": null,
        }))
        .send()
        .await
        .expect("create run");
    assert_eq!(response.status(), 201);

    let pre_run_hooks = json!([
        {
            "__meta": {
                "id": "capture-json",
                "config": { "path": "config/sat1.json" },
                "success": true,
                "error": null
            },
            "content": { "a": 1.0 }
        },
        {
            "__meta": {
                "id": "capture-json",
                "config": { "path": "config/sat2.json" },
                "success": true,
                "error": null
            },
            "content": { "a": 2.0 }
        }
    ]);

    let form = reqwest::multipart::Form::new()
        .text("run_id", run_id.to_string())
        .text("pre_run", pre_run_hooks.to_string());
    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("upload hooks");
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("parse upload body");
    assert_eq!(body["pre_run_hooks"], 2);

    let response = client
        .get(format!("{}/api/v1/runs/{run_id}", ctx.base_url()))
        .send()
        .await
        .expect("get run");
    let body: serde_json::Value = response.json().await.expect("parse body");
    let pre = body["pre_run_hooks"]
        .as_array()
        .expect("pre_run_hooks array");
    assert_eq!(pre.len(), 2, "both rows must persist");
}

#[tokio::test]
async fn multiple_hooks_with_identical_configs_coexist() {
    // Reviewer's concern that closed #1019: two hooks with the same
    // hook_id AND the same config (e.g., two capture-command hooks
    // running the identical command) should still produce distinct rows.
    // A config-hash discriminator would collide; hook_index uses array
    // position, so they remain separate.
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_id = "01HOOKIDX00000000000000B";
    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&json!({
            "id": run_id,
            "name": "multi-hooks-identical",
            "timestamp": "2026-06-13T10:00:00Z",
            "command": "test",
            "vault": "hook-index-vault",
            "project_root": "/tmp/test",
            "exit_code": 0,
            "duration_ms": 100,
            "stdout": null,
            "stderr": null,
        }))
        .send()
        .await
        .expect("create run");
    assert_eq!(response.status(), 201);

    let pre_run_hooks = json!([
        {
            "__meta": {
                "id": "capture-command",
                "config": { "command": "echo hello" },
                "success": true,
                "error": null
            },
            "stdout": "first invocation",
            "exit_code": 0
        },
        {
            "__meta": {
                "id": "capture-command",
                "config": { "command": "echo hello" },
                "success": true,
                "error": null
            },
            "stdout": "second invocation",
            "exit_code": 0
        }
    ]);

    let form = reqwest::multipart::Form::new()
        .text("run_id", run_id.to_string())
        .text("pre_run", pre_run_hooks.to_string());
    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("upload hooks");
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("parse body");
    assert_eq!(body["pre_run_hooks"], 2);

    let response = client
        .get(format!("{}/api/v1/runs/{run_id}", ctx.base_url()))
        .send()
        .await
        .expect("get run");
    let body: serde_json::Value = response.json().await.expect("parse body");
    let pre = body["pre_run_hooks"]
        .as_array()
        .expect("pre_run_hooks array");
    assert_eq!(pre.len(), 2);
    let stdouts: Vec<&str> = pre
        .iter()
        .map(|h| h["stdout"].as_str().expect("stdout"))
        .collect();
    assert!(
        stdouts.contains(&"first invocation"),
        "missing first invocation, got: {stdouts:?}"
    );
    assert!(
        stdouts.contains(&"second invocation"),
        "missing second invocation, got: {stdouts:?}"
    );
}

#[tokio::test]
async fn re_upload_with_same_array_is_idempotent() {
    // The hook_index is stable for re-uploads of the same capsula.toml
    // structure, so ON CONFLICT UPSERTs the existing row instead of
    // creating a duplicate.
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_id = "01HOOKIDX00000000000000C";
    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&json!({
            "id": run_id,
            "name": "re-upload-test",
            "timestamp": "2026-06-13T10:00:00Z",
            "command": "test",
            "vault": "hook-index-vault",
            "project_root": "/tmp/test",
            "exit_code": 0,
            "duration_ms": 100,
            "stdout": null,
            "stderr": null,
        }))
        .send()
        .await
        .expect("create run");
    assert_eq!(response.status(), 201);

    let upload = |variant: &str| {
        let body = json!([
            {
                "__meta": {
                    "id": "capture-json",
                    "config": { "path": "config/a.json" },
                    "success": true,
                    "error": null
                },
                "content": { "v": variant }
            },
            {
                "__meta": {
                    "id": "capture-json",
                    "config": { "path": "config/b.json" },
                    "success": true,
                    "error": null
                },
                "content": { "v": variant }
            }
        ]);
        reqwest::multipart::Form::new()
            .text("run_id", run_id.to_string())
            .text("pre_run", body.to_string())
    };

    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(upload("first"))
        .send()
        .await
        .expect("first upload");
    assert_eq!(response.status(), 200);

    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(upload("second"))
        .send()
        .await
        .expect("second upload");
    assert_eq!(response.status(), 200);

    let response = client
        .get(format!("{}/api/v1/runs/{run_id}", ctx.base_url()))
        .send()
        .await
        .expect("get run");
    let body: serde_json::Value = response.json().await.expect("parse body");
    let pre = body["pre_run_hooks"]
        .as_array()
        .expect("pre_run_hooks array");
    assert_eq!(pre.len(), 2, "re-upload must UPSERT, not duplicate");
    for hook in pre {
        assert_eq!(
            hook["content"]["v"], "second",
            "every row should reflect the latest upload"
        );
    }
}

#[tokio::test]
async fn response_exposes_hook_index_and_preserves_array_order() {
    // Verifies that each hook payload returned by GET /api/v1/runs/{id}
    // carries its capsula.toml array position under __meta.hook_index,
    // and that the response list is stably ordered by (phase, hook_index).
    let ctx = TestContext::new().await;
    let client = reqwest::Client::new();

    let run_id = "01HKIDXORDER00000000000AA";
    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&json!({
            "id": run_id,
            "name": "hook-index-order-test",
            "timestamp": "2026-06-24T10:00:00Z",
            "command": "test",
            "vault": "hook-index-order",
            "project_root": "/tmp/test",
            "exit_code": 0,
            "duration_ms": 100,
            "stdout": null,
            "stderr": null,
        }))
        .send()
        .await
        .expect("create run");
    assert_eq!(response.status(), 201);

    // Three hooks; positions 0 and 2 share hook_id "capture-command" so
    // hook_index is the only thing that distinguishes them.
    let pre_run_hooks = json!([
        {
            "__meta": {
                "id": "capture-command",
                "config": { "command": "echo first" },
                "success": true,
                "error": null
            },
            "stdout": "first"
        },
        {
            "__meta": {
                "id": "capture-json",
                "config": { "path": "middle.json" },
                "success": true,
                "error": null
            },
            "content": { "n": 1 }
        },
        {
            "__meta": {
                "id": "capture-command",
                "config": { "command": "echo second" },
                "success": true,
                "error": null
            },
            "stdout": "second"
        }
    ]);

    let form = reqwest::multipart::Form::new()
        .text("run_id", run_id.to_string())
        .text("pre_run", pre_run_hooks.to_string());
    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("upload");
    assert_eq!(response.status(), 200);

    let response = client
        .get(format!("{}/api/v1/runs/{run_id}", ctx.base_url()))
        .send()
        .await
        .expect("get run");
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("parse body");
    let pre = body["pre_run_hooks"]
        .as_array()
        .expect("pre_run_hooks array");
    assert_eq!(pre.len(), 3);

    // Ordering: hook_index must be 0, 1, 2 in that sequence.
    let indices: Vec<i64> = pre
        .iter()
        .map(|h| h["__meta"]["hook_index"].as_i64().expect("hook_index"))
        .collect();
    assert_eq!(indices, vec![0, 1, 2]);

    // Content-level ordering matches capsula.toml array order.
    assert_eq!(pre[0]["__meta"]["id"], "capture-command");
    assert_eq!(pre[0]["stdout"], "first");
    assert_eq!(pre[1]["__meta"]["id"], "capture-json");
    assert_eq!(pre[1]["content"]["n"], 1);
    assert_eq!(pre[2]["__meta"]["id"], "capture-command");
    assert_eq!(pre[2]["stdout"], "second");
}
// =============================================================================
// ParameterMatch integration tests
// =============================================================================
//
// Exercises POST /api/v1/runs/search with `parameter_matches`. The filter
// targets rows produced by parameter-capturing hooks (capture-json,
// capture-toml), which all share the shape:
//
//   __meta.config = { "path": "<configured-path>" }
//   output        = { "content": <parsed JSON> }
//
// The server selects rows structurally (`output ? 'content'`), pins by
// `config->>'path'` when `file` is set, and adds a JSONPath predicate
// rooted at `$.content` when `parameter`+`operator`+`value` is set.

async fn pm_create_run(ctx: &TestContext, run_id: &str, vault: &str) {
    let run_data = json!({
        "id": run_id,
        "name": format!("test-{run_id}"),
        "timestamp": "2026-01-08T10:00:00Z",
        "command": "test",
        "vault": vault,
        "project_root": "/tmp/test",
        "exit_code": 0,
        "duration_ms": 100,
        "stdout": null,
        "stderr": null,
    });
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v1/runs", ctx.base_url()))
        .json(&run_data)
        .send()
        .await
        .expect("create run");
    assert_eq!(response.status(), 201, "create_run for {run_id} failed");
}

async fn pm_upload_hook(
    ctx: &TestContext,
    run_id: &str,
    phase: &str,
    hook_id: &str,
    config: serde_json::Value,
    payload: serde_json::Value,
) {
    let mut combined = serde_json::Map::new();
    combined.insert(
        "__meta".to_string(),
        json!({
            "id": hook_id,
            "config": config,
            "success": true,
            "error": null,
        }),
    );
    if let Some(obj) = payload.as_object() {
        for (k, v) in obj {
            combined.insert(k.clone(), v.clone());
        }
    }
    let hooks_array = serde_json::Value::Array(vec![serde_json::Value::Object(combined)]);
    let field = match phase {
        "pre" => "pre_run",
        "post" => "post_run",
        other => panic!("phase must be 'pre' or 'post', got: {other}"),
    };
    let form = reqwest::multipart::Form::new()
        .text("run_id", run_id.to_string())
        .text(field, hooks_array.to_string());
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("upload hook");
    assert_eq!(
        response.status(),
        200,
        "upload {phase} hook for {run_id} failed"
    );
}

async fn pm_seed_capture_json(
    ctx: &TestContext,
    run_id: &str,
    vault: &str,
    phase: &str,
    file: &str,
    content: serde_json::Value,
) {
    pm_create_run(ctx, run_id, vault).await;
    pm_upload_hook(
        ctx,
        run_id,
        phase,
        "capture-json",
        json!({ "path": file }),
        json!({ "content": content }),
    )
    .await;
}

async fn pm_search(ctx: &TestContext, body: serde_json::Value) -> serde_json::Value {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v1/runs/search", ctx.base_url()))
        .json(&body)
        .send()
        .await
        .expect("search request");
    assert_eq!(response.status(), 200);
    response.json().await.expect("parse body")
}

fn pm_run_ids(body: &serde_json::Value) -> Vec<String> {
    body["runs"]
        .as_array()
        .expect("runs array")
        .iter()
        .map(|r| r["id"].as_str().expect("id").to_string())
        .collect()
}

#[tokio::test]
async fn pm_finds_run_by_file_and_parameter() {
    let ctx = TestContext::new().await;

    pm_seed_capture_json(
        &ctx,
        "01PMI001AAAAAAAAAAAAAAA",
        "v1",
        "pre",
        "config.json",
        json!({ "lr": 0.01 }),
    )
    .await;
    pm_seed_capture_json(
        &ctx,
        "01PMI001BBBBBBBBBBBBBBB",
        "v1",
        "pre",
        "config.json",
        json!({ "lr": 0.001 }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "file": "config.json",
                "parameter": "lr",
                "operator": "ge",
                "value": 0.01,
            }]
        }),
    )
    .await;

    assert_eq!(body["status"], "ok");
    assert_eq!(body["total"], 1);
    assert_eq!(
        pm_run_ids(&body),
        vec!["01PMI001AAAAAAAAAAAAAAA".to_string()]
    );
}

#[tokio::test]
async fn pm_string_eq() {
    let ctx = TestContext::new().await;

    pm_seed_capture_json(
        &ctx,
        "01PMI002STRAAAAAAAAAAAA",
        "v1",
        "pre",
        "model.json",
        json!({ "architecture": "transformer" }),
    )
    .await;
    pm_seed_capture_json(
        &ctx,
        "01PMI002STRBBBBBBBBBBBB",
        "v1",
        "pre",
        "model.json",
        json!({ "architecture": "lstm" }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "file": "model.json",
                "parameter": "architecture",
                "operator": "eq",
                "value": "transformer",
            }]
        }),
    )
    .await;

    assert_eq!(body["total"], 1);
    assert_eq!(
        pm_run_ids(&body),
        vec!["01PMI002STRAAAAAAAAAAAA".to_string()]
    );
}

#[tokio::test]
async fn pm_nested_dot_path() {
    let ctx = TestContext::new().await;

    pm_seed_capture_json(
        &ctx,
        "01PMI003NESTAAAAAAAAAAA",
        "v1",
        "pre",
        "sat1/orbit.json",
        json!({ "orbit": { "a": 1.42 } }),
    )
    .await;
    pm_seed_capture_json(
        &ctx,
        "01PMI003NESTBBBBBBBBBBB",
        "v1",
        "pre",
        "sat1/orbit.json",
        json!({ "orbit": { "a": 0.5 } }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "file": "sat1/orbit.json",
                "parameter": "orbit.a",
                "operator": "ge",
                "value": 1.0,
            }]
        }),
    )
    .await;

    assert_eq!(body["total"], 1);
    assert_eq!(
        pm_run_ids(&body),
        vec!["01PMI003NESTAAAAAAAAAAA".to_string()]
    );
}

#[tokio::test]
async fn pm_multiple_matches_are_anded_across_files() {
    let ctx = TestContext::new().await;

    // Run A: two capture-json hooks with the same hook_id but distinct
    // config.path. With the hook_index widening of the run_outputs unique
    // index (#1017 / #1036), they coexist as separate rows.
    //
    // NOTE: capsula's `/api/v1/upload` enumerates each phase's array with
    // `.enumerate()` and uses the position as hook_index. Calling
    // `pm_upload_hook` twice in a row each sends a 1-element array, so
    // both rows would receive hook_index=0 and the second upload would
    // ON CONFLICT UPSERT the first. Send both hooks in a single multipart
    // POST instead — that mirrors how the real CLI uploads pre-run.json.
    pm_create_run(&ctx, "01PMI004ANDOKAAAAAAAAAA", "v1").await;
    let pre_run_hooks = json!([
        {
            "__meta": {
                "id": "capture-json",
                "config": { "path": "train.json" },
                "success": true,
                "error": null
            },
            "content": { "lr": 0.01 }
        },
        {
            "__meta": {
                "id": "capture-json",
                "config": { "path": "data.json" },
                "success": true,
                "error": null
            },
            "content": { "batch_size": 32 }
        }
    ]);
    let form = reqwest::multipart::Form::new()
        .text("run_id", "01PMI004ANDOKAAAAAAAAAA".to_string())
        .text("pre_run", pre_run_hooks.to_string());
    let response = reqwest::Client::new()
        .post(format!("{}/api/v1/upload", ctx.base_url()))
        .multipart(form)
        .send()
        .await
        .expect("batch upload");
    assert_eq!(response.status(), 200);

    pm_seed_capture_json(
        &ctx,
        "01PMI004ANDNOBBBBBBBBBB",
        "v1",
        "pre",
        "train.json",
        json!({ "lr": 0.01 }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [
                { "phase": "pre", "file": "train.json", "parameter": "lr",
                  "operator": "ge", "value": 0.001 },
                { "phase": "pre", "file": "data.json", "parameter": "batch_size",
                  "operator": "eq", "value": 32 }
            ]
        }),
    )
    .await;

    assert_eq!(body["total"], 1);
    assert_eq!(
        pm_run_ids(&body),
        vec!["01PMI004ANDOKAAAAAAAAAA".to_string()]
    );
}

#[tokio::test]
async fn pm_range_query_on_same_file() {
    let ctx = TestContext::new().await;

    pm_seed_capture_json(
        &ctx,
        "01PMI005RNG001AAAAAAAAA",
        "v1",
        "pre",
        "train.json",
        json!({ "lr": 0.001 }),
    )
    .await;
    pm_seed_capture_json(
        &ctx,
        "01PMI005RNG010BBBBBBBBB",
        "v1",
        "pre",
        "train.json",
        json!({ "lr": 0.01 }),
    )
    .await;
    pm_seed_capture_json(
        &ctx,
        "01PMI005RNG100CCCCCCCCC",
        "v1",
        "pre",
        "train.json",
        json!({ "lr": 0.1 }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [
                { "phase": "pre", "file": "train.json", "parameter": "lr",
                  "operator": "ge", "value": 0.005 },
                { "phase": "pre", "file": "train.json", "parameter": "lr",
                  "operator": "le", "value": 0.05 }
            ]
        }),
    )
    .await;

    assert_eq!(body["total"], 1);
    assert_eq!(
        pm_run_ids(&body),
        vec!["01PMI005RNG010BBBBBBBBB".to_string()]
    );
}

#[tokio::test]
async fn pm_phase_pre_does_not_match_post_outputs() {
    let ctx = TestContext::new().await;

    pm_seed_capture_json(
        &ctx,
        "01PMI006PHASEAAAAAAAAAA",
        "v1",
        "post",
        "config.json",
        json!({ "lr": 0.01 }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "file": "config.json",
                "parameter": "lr",
                "operator": "eq",
                "value": 0.01,
            }]
        }),
    )
    .await;

    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn pm_does_not_match_non_parameter_hooks() {
    let ctx = TestContext::new().await;

    // capture-env style: no `content` field. ParameterMatch's structural
    // filter (`output ? 'content'`) must skip it.
    pm_create_run(&ctx, "01PMI007OTHRAAAAAAAAAAA", "v1").await;
    pm_upload_hook(
        &ctx,
        "01PMI007OTHRAAAAAAAAAAA",
        "pre",
        "capture-env",
        json!({ "name": "PATH" }),
        json!({ "value": "/usr/bin" }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "parameter": "value",
                "operator": "eq",
                "value": "/usr/bin",
            }]
        }),
    )
    .await;

    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn pm_file_only_matches_when_file_present() {
    let ctx = TestContext::new().await;

    pm_seed_capture_json(
        &ctx,
        "01PMI008FOAAAAAAAAAAAAA",
        "v1",
        "pre",
        "orbit.json",
        json!({ "a": 1 }),
    )
    .await;
    pm_seed_capture_json(
        &ctx,
        "01PMI008FOBBBBBBBBBBBBB",
        "v1",
        "pre",
        "other.json",
        json!({ "a": 1 }),
    )
    .await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "file": "orbit.json",
            }]
        }),
    )
    .await;

    assert_eq!(body["total"], 1);
    assert_eq!(
        pm_run_ids(&body),
        vec!["01PMI008FOAAAAAAAAAAAAA".to_string()]
    );
}

#[tokio::test]
async fn pm_invalid_phase_returns_error() {
    let ctx = TestContext::new().await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "during",
                "file": "config.json",
            }]
        }),
    )
    .await;

    assert_eq!(body["status"], "error", "body: {body}");
}

#[tokio::test]
async fn pm_empty_match_returns_error() {
    let ctx = TestContext::new().await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
            }]
        }),
    )
    .await;

    assert_eq!(body["status"], "error", "body: {body}");
}

#[tokio::test]
async fn pm_partial_triple_returns_error() {
    let ctx = TestContext::new().await;

    let body = pm_search(
        &ctx,
        json!({
            "vault": "v1",
            "parameter_matches": [{
                "phase": "pre",
                "file": "config.json",
                "parameter": "lr",
                "operator": "ge"
                // value missing
            }]
        }),
    )
    .await;

    assert_eq!(body["status"], "error", "body: {body}");
}
