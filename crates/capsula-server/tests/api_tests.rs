#![allow(clippy::unwrap_used, reason = "Test code")]
use serde_json::json;

const BASE_URL: &str = "http://127.0.0.1:3000";

#[tokio::test]
async fn test_health_check() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{BASE_URL}/health"))
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
    let client = reqwest::Client::new();

    // Create a run
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
        .post(format!("{BASE_URL}/api/runs"))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "created");
    assert_eq!(body["run"]["id"], "01TEST123456789ABCDEFGHIJ");

    // Get the run
    let response = client
        .get(format!("{BASE_URL}/api/runs/01TEST123456789ABCDEFGHIJ"))
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
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{BASE_URL}/api/vaults"))
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
    let client = reqwest::Client::new();

    // Check existing vault
    let response = client
        .get(format!("{BASE_URL}/api/vaults/default"))
        .send()
        .await
        .expect("Failed to get vault");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");

    // Check non-existing vault
    let response = client
        .get(format!("{BASE_URL}/api/vaults/nonexistent-vault-xyz"))
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
    let client = reqwest::Client::new();

    // List all runs
    let response = client
        .get(format!("{BASE_URL}/api/runs"))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert!(body["runs"].is_array());

    // List with vault filter
    let response = client
        .get(format!("{BASE_URL}/api/runs?vault=default"))
        .send()
        .await
        .expect("Failed to list runs");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");

    // All runs should be from default vault
    if let Some(runs) = body["runs"].as_array() {
        for run in runs {
            assert_eq!(run["vault"], "default");
        }
    }
}

#[tokio::test]
async fn test_pagination() {
    let client = reqwest::Client::new();

    // Test with limit
    let response = client
        .get(format!("{BASE_URL}/api/runs?limit=2"))
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

    // Test with offset
    let response = client
        .get(format!("{BASE_URL}/api/runs?limit=1&offset=1"))
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
    let client = reqwest::Client::new();

    // Create a multipart form with test files
    let form = reqwest::multipart::Form::new()
        .text("file1", "content of file 1")
        .text("file2", "content of file 2");

    let response = client
        .post(format!("{BASE_URL}/api/upload"))
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
    let client = reqwest::Client::new();

    // First create a run to associate the file with
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
        .post(format!("{BASE_URL}/api/runs"))
        .json(&run_data)
        .send()
        .await
        .expect("Failed to create run");
    assert_eq!(response.status(), 200);

    // Upload a file associated with this run
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
        .post(format!("{BASE_URL}/api/upload"))
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
