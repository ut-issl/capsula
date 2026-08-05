//! Integration tests for `pull_single_run` against a mocked Capsula server.
#![cfg(test)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Tests use unwrap/expect for brevity"
)]

use capsula_core::run::run_dir_relative_path;
use capsula_core::util::hex_encode;
use capsula_orchestration::pull::pull_single_run;
use capsula_orchestration::push::push_single_run;
use capsula_orchestration::vault::{find_run_dir, list_runs};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use ulid::Ulid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const RUN_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const RUN_NAME: &str = "pulled-run";
const VAULT: &str = "test-vault";
const FILE_CONTENT: &[u8] = b"hello from capsula";

fn sha256_hex(data: &[u8]) -> String {
    hex_encode(Sha256::digest(data).as_ref())
}

fn run_record() -> serde_json::Value {
    json!({
        "id": RUN_ID,
        "name": RUN_NAME,
        "timestamp": "2026-08-01T10:20:00Z",
        "command": "[\"echo\",\"hello\"]",
        "vault": VAULT,
        "project_root": "/origin/project",
        "exit_code": 0,
        "duration_ms": 1234,
        "stdout": "hello\n",
        "stderr": "",
        "created_at": "2026-08-01T10:20:01Z",
        "updated_at": "2026-08-01T10:20:01Z"
    })
}

fn detail_response(files: &serde_json::Value) -> serde_json::Value {
    detail_response_with_run(&run_record(), files)
}

fn detail_response_with_run(
    run: &serde_json::Value,
    files: &serde_json::Value,
) -> serde_json::Value {
    json!({
        "status": "ok",
        "run": run,
        "pre_run_hooks": [
            {
                "__meta": {
                    "id": "capture-cwd",
                    "config": {},
                    "success": true,
                    "error": null,
                    "hook_index": 0
                },
                "cwd": "/origin/project"
            }
        ],
        "post_run_hooks": [],
        "files": files
    })
}

/// Start a mock server (kept alive by the returned runtime) serving the
/// given run-detail response and one downloadable file.
///
/// `encoded_file_path` is the percent-encoded URL form of the file path
/// (wiremock matches the encoded request path; the real server decodes it).
fn start_server(
    detail: serde_json::Value,
    encoded_file_path: Option<&str>,
) -> (tokio::runtime::Runtime, MockServer) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/runs/{RUN_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(detail))
            .mount(&server)
            .await;
        if let Some(fp) = encoded_file_path {
            Mock::given(method("GET"))
                .and(path(format!("/api/v1/runs/{RUN_ID}/files/{fp}")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(FILE_CONTENT.to_vec()))
                .mount(&server)
                .await;
        }
        server
    });
    (rt, server)
}

fn expected_run_dir(vault_dir: &Path) -> PathBuf {
    let ulid = Ulid::from_string(RUN_ID).unwrap();
    vault_dir.join(run_dir_relative_path(&ulid, RUN_NAME))
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn pull_restores_run_with_files_and_metadata() {
    let files = json!([
        {"path": "output/result.txt", "size": FILE_CONTENT.len(), "hash": sha256_hex(FILE_CONTENT)}
    ]);
    let (_rt, server) = start_server(detail_response(&files), Some("output/result.txt"));
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let run_dir = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap();

    assert_eq!(run_dir, expected_run_dir(&vault_dir));

    // Captured file restored byte-exact
    assert_eq!(
        std::fs::read(run_dir.join("output/result.txt")).unwrap(),
        FILE_CONTENT
    );

    // metadata.json reconstructed with the original field layout
    let metadata = read_json(&run_dir.join("_capsula/metadata.json"));
    assert_eq!(metadata["id"], RUN_ID);
    assert_eq!(metadata["name"], RUN_NAME);
    assert_eq!(metadata["command"], json!(["echo", "hello"]));
    assert_eq!(metadata["run_dir"], run_dir.to_string_lossy().as_ref());
    assert_eq!(metadata["project_root"], "/origin/project");

    // Hook outputs restored; empty post-run phase produces no file
    let pre_run = read_json(&run_dir.join("_capsula/pre-run.json"));
    assert_eq!(pre_run[0]["__meta"]["id"], "capture-cwd");
    assert_eq!(pre_run[0]["cwd"], "/origin/project");
    assert!(!run_dir.join("_capsula/post-run.json").exists());

    // command.json reconstructed from exit_code / duration_ms / stdout
    let command = read_json(&run_dir.join("_capsula/command.json"));
    assert_eq!(command["exit_code"], 0);
    assert_eq!(command["stdout"], "hello\n");
    assert_eq!(command["duration"]["secs"], 1);
    assert_eq!(command["duration"]["nanos"], 234_000_000);

    // Marker records the origin
    let pulled = read_json(&run_dir.join("_capsula/pulled.json"));
    assert_eq!(pulled["server_url"], server.uri());
    assert!(pulled["pulled_at"].is_string());
}

#[test]
fn pull_fails_on_hash_mismatch_without_leaving_partial_run() {
    let files = json!([
        {"path": "output/result.txt", "size": FILE_CONTENT.len(), "hash": sha256_hex(b"different content")}
    ]);
    let (_rt, server) = start_server(detail_response(&files), Some("output/result.txt"));
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap_err();

    assert!(err.to_string().contains("Hash mismatch"), "got: {err}");
    assert!(!expected_run_dir(&vault_dir).exists());
}

#[test]
fn pull_fails_on_size_mismatch() {
    let files = json!([
        {"path": "output/result.txt", "size": FILE_CONTENT.len() + 1, "hash": sha256_hex(FILE_CONTENT)}
    ]);
    let (_rt, server) = start_server(detail_response(&files), Some("output/result.txt"));
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap_err();

    assert!(err.to_string().contains("Size mismatch"), "got: {err}");
    assert!(!expected_run_dir(&vault_dir).exists());
}

#[test]
fn pull_restores_file_without_recorded_hash() {
    // Files without a stored hash are restored (with a warning) rather
    // than rejected.
    let files = json!([
        {"path": "nohash.txt", "size": FILE_CONTENT.len(), "hash": null}
    ]);
    let (_rt, server) = start_server(detail_response(&files), Some("nohash.txt"));
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let run_dir = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap();

    assert_eq!(
        std::fs::read(run_dir.join("nohash.txt")).unwrap(),
        FILE_CONTENT
    );
}

#[test]
fn pull_rejects_vault_mismatch() {
    let (_rt, server) = start_server(detail_response(&json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join("other-vault");

    let err = pull_single_run(RUN_ID, "other-vault", &vault_dir, &client, false).unwrap_err();

    assert!(err.to_string().contains("belongs to vault"), "got: {err}");
    assert!(!expected_run_dir(&vault_dir).exists());
}

#[test]
fn pull_rejects_unsafe_file_paths() {
    let files = json!([
        {"path": "../evil.txt", "size": FILE_CONTENT.len(), "hash": sha256_hex(FILE_CONTENT)}
    ]);
    let (_rt, server) = start_server(detail_response(&files), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap_err();

    assert!(err.to_string().contains("unsafe path"), "got: {err}");
    assert!(!tmp.path().join("evil.txt").exists());
}

#[test]
fn pull_rejects_run_name_that_is_not_a_single_path_component() {
    let mut run = run_record();
    run["name"] = json!("../evil");
    let (_rt, server) = start_server(detail_response_with_run(&run, &json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap_err();

    assert!(
        err.to_string().contains("single path component"),
        "got: {err}"
    );
    assert!(!tmp.path().join("evil").exists());
}

#[test]
fn pull_rejects_run_names_as_argument() {
    // A run *name* must fail fast with a specific message instead of a
    // misleading server-side not_found.
    let (_rt, server) = start_server(detail_response(&json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();

    let err = pull_single_run(
        "happy-river",
        VAULT,
        &tmp.path().join(VAULT),
        &client,
        false,
    )
    .unwrap_err();

    assert!(err.to_string().contains("not a run ID"), "got: {err}");
}

#[test]
fn pull_refuses_to_replace_locally_produced_run() {
    let (_rt, server) = start_server(detail_response(&json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    // A locally produced run: has _capsula but no pulled.json marker.
    let run_dir = expected_run_dir(&vault_dir);
    std::fs::create_dir_all(run_dir.join("_capsula")).unwrap();
    std::fs::write(run_dir.join("_capsula/metadata.json"), "{}").unwrap();
    std::fs::write(run_dir.join("stale.txt"), "authoritative").unwrap();

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap_err();
    assert!(err.to_string().contains("locally produced"), "got: {err}");

    // Even --force must not replace a locally produced run.
    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, true).unwrap_err();
    assert!(
        err.to_string()
            .contains("--force only replaces previously pulled runs"),
        "got: {err}"
    );
    assert!(
        run_dir.join("stale.txt").exists(),
        "local run must be untouched"
    );
}

#[test]
fn pull_force_replaces_previously_pulled_run() {
    let (_rt, server) = start_server(detail_response(&json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let run_dir = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap();
    std::fs::write(run_dir.join("extra.txt"), "local modification").unwrap();

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap_err();
    assert!(err.to_string().contains("Use --force"), "got: {err}");
    assert!(run_dir.join("extra.txt").exists());

    let pulled_dir = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, true).unwrap();
    assert_eq!(pulled_dir, run_dir);
    assert!(!run_dir.join("extra.txt").exists(), "--force replaces");
    assert!(run_dir.join("_capsula/pulled.json").exists());
}

#[test]
fn pull_force_refuses_when_target_is_a_file() {
    let (_rt, server) = start_server(detail_response(&json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let run_dir = expected_run_dir(&vault_dir);
    std::fs::create_dir_all(run_dir.parent().unwrap()).unwrap();
    std::fs::write(&run_dir, "not a directory").unwrap();

    let err = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, true).unwrap_err();

    assert!(
        err.to_string()
            .contains("--force only replaces previously pulled runs"),
        "got: {err}"
    );
    assert!(run_dir.is_file(), "existing file must be untouched");
}

#[test]
fn push_refuses_pulled_runs() {
    let (_rt, server) = start_server(detail_response(&json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let run_dir = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap();

    // The guard fires before any network access, so an unreachable
    // server URL proves it.
    let offline = capsula_client::CapsulaClient::new("http://127.0.0.1:9");
    let err = push_single_run(&run_dir, VAULT, "http://127.0.0.1:9", &offline).unwrap_err();

    assert!(err.to_string().contains("refusing to push"), "got: {err}");
}

#[test]
fn abandoned_temp_dir_is_invisible_to_vault_scanners() {
    // Simulates a pull interrupted after the _capsula reconstruction:
    // the temp directory lives directly under the vault root, one level
    // above the `vault/*/*` depth the scanners walk.
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);
    let stale = vault_dir.join(".pull-tmp-abandoned");
    std::fs::create_dir_all(stale.join("_capsula")).unwrap();
    std::fs::write(
        stale.join("_capsula/metadata.json"),
        json!({
            "id": RUN_ID,
            "name": "ghost",
            "command": [],
            "timestamp": "2026-08-01T10:20:00Z"
        })
        .to_string(),
    )
    .unwrap();

    assert!(
        list_runs(&vault_dir).unwrap().is_empty(),
        "abandoned temp dir must not be listed as a run"
    );
    assert!(
        find_run_dir(&vault_dir, RUN_ID).is_err(),
        "abandoned temp dir must not resolve as a run"
    );
}

#[test]
fn run_without_exit_code_produces_no_command_json() {
    // A run created by `run-start` and never finalized has no command
    // result on the server; the restored run must not invent one.
    let mut run = run_record();
    run["exit_code"] = json!(null);
    run["duration_ms"] = json!(null);
    run["stdout"] = json!(null);
    run["stderr"] = json!(null);
    let (_rt, server) = start_server(detail_response_with_run(&run, &json!([])), None);
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();

    let run_dir = pull_single_run(RUN_ID, VAULT, &tmp.path().join(VAULT), &client, false).unwrap();

    assert!(!run_dir.join("_capsula/command.json").exists());
    assert!(run_dir.join("_capsula/metadata.json").exists());
}

#[test]
fn pull_restores_file_with_url_significant_characters_in_name() {
    // Spaces (and other URL-significant characters) are legal in captured
    // file paths. The mock below is mounted at the percent-encoded path
    // only: if the client sent such a path unencoded, the request would
    // miss the mock entirely.
    let files = json!([
        {"path": "nested dir/file with space.txt", "size": FILE_CONTENT.len(), "hash": sha256_hex(FILE_CONTENT)}
    ]);
    let (_rt, server) = start_server(
        detail_response(&files),
        Some("nested%20dir/file%20with%20space.txt"),
    );
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();
    let vault_dir = tmp.path().join(VAULT);

    let run_dir = pull_single_run(RUN_ID, VAULT, &vault_dir, &client, false).unwrap();

    assert_eq!(
        std::fs::read(run_dir.join("nested dir/file with space.txt")).unwrap(),
        FILE_CONTENT
    );
}

#[test]
fn pull_reports_run_not_found() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/runs/{RUN_ID}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "not_found",
                "error": format!("Run with id {RUN_ID} not found")
            })))
            .mount(&server)
            .await;
        server
    });
    let client = capsula_client::CapsulaClient::new(server.uri());
    let tmp = tempfile::TempDir::new().unwrap();

    let err = pull_single_run(RUN_ID, VAULT, &tmp.path().join(VAULT), &client, false).unwrap_err();

    assert!(err.to_string().contains("not found"), "got: {err}");
}
