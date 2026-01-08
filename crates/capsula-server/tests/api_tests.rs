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
