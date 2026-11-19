//! End-to-end push flow tests against a mock server.
//!
//! Every request of the push sequence must carry the configured headers;
//! the header matchers make the mock reject unauthenticated requests.
#![cfg(test)]

use capsula_client::CapsulaClient;
use capsula_orchestration::push::{PushAllSummary, PushOutcome, push_single_run};
use std::path::PathBuf;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_run_fixture(run_dir: &std::path::Path) {
    write_named_run_fixture(run_dir, "01TESTPUSHFLOWRUN", "test-run");
}

#[tokio::test(flavor = "multi_thread")]
async fn push_sends_configured_headers_on_every_request() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/runs"))
        .and(header("x-auth", "secret"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/upload"))
        .and(header("x-auth", "secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "files_processed": 1,
            "total_bytes": 4,
            "pre_run_hooks": 0,
            "post_run_hooks": 0,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let run_dir = tempfile::tempdir().unwrap();
    write_run_fixture(run_dir.path());

    let uri = server.uri();
    let run_path: PathBuf = run_dir.path().to_path_buf();
    let outcome = tokio::task::spawn_blocking(move || {
        let client =
            CapsulaClient::with_headers(uri, &[("x-auth".to_string(), "secret".to_string())])
                .unwrap();
        push_single_run(&run_path, "test-vault", &client)
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(outcome, PushOutcome::Created);
    // .expect(1) on each mock verifies both endpoints were hit (with headers)
    // when the MockServer is dropped
}

fn write_named_run_fixture(run_dir: &std::path::Path, id: &str, name: &str) {
    let capsula_dir = run_dir.join("_capsula");
    std::fs::create_dir_all(&capsula_dir).unwrap();
    std::fs::write(
        capsula_dir.join("metadata.json"),
        serde_json::json!({
            "id": id,
            "name": name,
            "timestamp": "2026-08-03T00:00:00Z",
            "command": ["echo", "hello"],
            "project_root": "/tmp/project",
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(run_dir.join("output.txt"), "data").unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn push_batch_distinguishes_created_and_existing_runs() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/runs"))
        .and(body_partial_json(serde_json::json!({ "id": "01NEWRUN" })))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/runs"))
        .and(body_partial_json(
            serde_json::json!({ "id": "01EXISTINGRUN" }),
        ))
        .respond_with(ResponseTemplate::new(409))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "files_processed": 1,
            "total_bytes": 4,
            "pre_run_hooks": 0,
            "post_run_hooks": 0,
        })))
        .expect(2)
        .mount(&server)
        .await;

    let vault = tempfile::tempdir().unwrap();
    let new_run = vault.path().join("new");
    let existing_run = vault.path().join("existing");
    write_named_run_fixture(&new_run, "01NEWRUN", "new-run");
    write_named_run_fixture(&existing_run, "01EXISTINGRUN", "existing-run");

    let uri = server.uri();
    let summary = tokio::task::spawn_blocking(move || {
        let client = CapsulaClient::new(uri);
        let mut summary = PushAllSummary::default();
        for run_dir in [&new_run, &existing_run] {
            match push_single_run(run_dir, "test-vault", &client) {
                Ok(outcome) => summary.record(outcome),
                Err(_) => summary.record_failure(),
            }
        }
        summary
    })
    .await
    .unwrap();

    assert_eq!(
        summary,
        PushAllSummary {
            created: 1,
            existing: 1,
            skipped_pulled: 0,
            failed: 0,
        }
    );
    assert_eq!(
        summary.to_string(),
        "Push all completed: 1 created, 1 re-uploaded, 0 skipped (pulled runs), 0 failed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn push_fails_when_upload_fails_after_conflict() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/runs"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/upload"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let run_dir = tempfile::tempdir().unwrap();
    write_run_fixture(run_dir.path());

    let uri = server.uri();
    let run_path: PathBuf = run_dir.path().to_path_buf();
    let result = tokio::task::spawn_blocking(move || {
        let client = CapsulaClient::new(uri);
        push_single_run(&run_path, "test-vault", &client)
    })
    .await
    .unwrap();

    assert!(result.is_err(), "got: {result:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn push_fails_with_auth_hint_when_run_creation_is_rejected() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/runs"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let run_dir = tempfile::tempdir().unwrap();
    write_run_fixture(run_dir.path());

    let uri = server.uri();
    let run_path: PathBuf = run_dir.path().to_path_buf();
    let error = tokio::task::spawn_blocking(move || {
        let client = CapsulaClient::new(uri);
        push_single_run(&run_path, "test-vault", &client)
    })
    .await
    .unwrap()
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("authentication may be missing or expired")
    );
}
