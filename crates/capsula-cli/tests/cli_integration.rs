//! Integration tests for the capsula CLI tool.
#![expect(
    clippy::unwrap_used,
    reason = "Test code doesn't need production-level error handling"
)]

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a minimal capsula.toml config
fn create_test_config(dir: &TempDir, vault_name: &str) -> PathBuf {
    let config_path = dir.path().join("capsula.toml");
    let config_content = format!(
        r#"
[vault]
name = "{vault_name}"

[[pre-run.hooks]]
id = "capture-cwd"
"#,
    );
    fs::write(&config_path, config_content).unwrap();
    config_path
}

/// Helper to create a capsula.toml config with both pre-run and post-run hooks
fn create_test_config_with_post_run(dir: &TempDir, vault_name: &str) -> PathBuf {
    let config_path = dir.path().join("capsula.toml");
    let config_content = format!(
        r#"
[vault]
name = "{vault_name}"

[[pre-run.hooks]]
id = "capture-cwd"

[[post-run.hooks]]
id = "capture-cwd"
"#,
    );
    fs::write(&config_path, config_content).unwrap();
    config_path
}

#[derive(Deserialize)]
struct TestRunMetadata {
    name: String,
}

fn find_first_run(vault_dir: &std::path::Path) -> (PathBuf, String) {
    let date_dirs: Vec<_> = fs::read_dir(vault_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();

    for date_dir in date_dirs {
        let run_dirs: Vec<_> = fs::read_dir(date_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .collect();

        for run_dir in run_dirs {
            let run_path = run_dir.path();
            let metadata_path = run_path.join("_capsula").join("metadata.json");
            if !metadata_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&metadata_path).unwrap();
            let metadata: TestRunMetadata = serde_json::from_str(&content).unwrap();
            return (run_path, metadata.name);
        }
    }

    panic!("No run directory found");
}

#[test]
fn test_capsula_run_creates_run_directory() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("hello");

    cmd.assert().success();

    // Check that the vault directory was created
    let vault_dir = temp_dir.path().join(".capsula").join("test-vault");
    assert!(vault_dir.exists(), "Vault directory should exist");

    // Check that at least one run directory exists
    let date_dirs: Vec<_> = fs::read_dir(&vault_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();

    assert!(
        !date_dirs.is_empty(),
        "Should have at least one date directory"
    );

    // Find a run directory
    for date_dir in date_dirs {
        let run_dirs: Vec<_> = fs::read_dir(date_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .collect();

        if !run_dirs.is_empty() {
            let run_dir = run_dirs[0].path();
            let capsula_dir = run_dir.join("_capsula");
            assert!(capsula_dir.exists(), "_capsula directory should exist");
            assert!(
                capsula_dir.join("metadata.json").exists(),
                "metadata.json should exist"
            );
            assert!(
                capsula_dir.join("pre-run.json").exists(),
                "pre-run.json should exist"
            );
            assert!(
                capsula_dir.join("command.json").exists(),
                "command.json should exist"
            );
            return;
        }
    }

    panic!("No run directory found");
}

#[test]
fn test_capsula_run_dir_prints_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("hello");

    cmd.assert().success();

    let vault_dir = temp_dir.path().join(".capsula").join("test-vault");
    let (run_dir, run_name) = find_first_run(&vault_dir);

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-dir")
        .arg(&run_name);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(run_dir.to_string_lossy().as_ref()));
}

#[test]
fn test_capsula_list_shows_runs() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    // First, create a run
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert().success();

    // Now list runs
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TIMESTAMP"))
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("COMMAND"));
}

#[test]
fn test_capsula_push_requires_server_url() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    // First, create a run
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert().success();

    // Try to push without server URL (should fail)
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("push")
        .arg("01234567890123456789012345"); // Some fake ID

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Server URL not specified"));
}

#[test]
fn test_capsula_vaults_list_requires_server_url() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    // Try to list vaults without server URL (should fail)
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("vaults")
        .arg("list");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Server URL not specified"));
}

#[test]
fn test_capsula_run_with_nonexistent_config() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg("nonexistent.toml")
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Configuration file not found"));
}

#[test]
fn test_capsula_run_without_command() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No command specified"));
}

#[test]
fn test_capsula_config_with_server_url() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("capsula.toml");
    let config_content = r#"
[vault]
name = "test-vault"

server = "http://localhost:8500"

[[pre-run.hooks]]
id = "capture-cwd"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create a run to ensure config is valid
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert().success();
}

#[test]
fn test_run_start_creates_directory_and_pre_run() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_with_post_run(&temp_dir, "test-vault");

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-start");

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let run_name = stdout.trim();
    assert!(
        !run_name.is_empty(),
        "run-start should print run name to stdout"
    );

    let vault_dir = temp_dir.path().join(".capsula").join("test-vault");
    assert!(vault_dir.exists(), "Vault directory should exist");

    let (run_dir, found_name) = find_first_run(&vault_dir);
    assert_eq!(found_name, run_name);

    let capsula_dir = run_dir.join("_capsula");
    assert!(
        capsula_dir.join("metadata.json").exists(),
        "metadata.json should exist"
    );
    assert!(
        capsula_dir.join("pre-run.json").exists(),
        "pre-run.json should exist"
    );
    assert!(
        !capsula_dir.join("command.json").exists(),
        "command.json should NOT exist"
    );
    assert!(
        !capsula_dir.join("post-run.json").exists(),
        "post-run.json should NOT exist"
    );
}

#[test]
fn test_run_start_prints_name_to_stdout() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-start");

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "stdout should contain exactly one line (the run name)"
    );
    // Run names from the `names` crate are two hyphenated words
    assert!(
        lines[0].contains('-'),
        "Run name should be hyphenated (e.g., 'happy-river')"
    );
}

#[test]
fn test_run_end_creates_post_run() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_with_post_run(&temp_dir, "test-vault");

    // First, start a run
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-start");

    let output = cmd.assert().success();
    let run_name = String::from_utf8(output.get_output().stdout.clone())
        .unwrap()
        .trim()
        .to_string();

    // Now end the run
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-end")
        .arg(&run_name);

    cmd.assert().success();

    let vault_dir = temp_dir.path().join(".capsula").join("test-vault");
    let (run_dir, _) = find_first_run(&vault_dir);
    let capsula_dir = run_dir.join("_capsula");
    assert!(
        capsula_dir.join("post-run.json").exists(),
        "post-run.json should exist after run-end"
    );
}

#[test]
fn test_run_end_nonexistent_name_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-end")
        .arg("nonexistent-run");

    cmd.assert().failure();
}

#[test]
fn test_run_end_already_finalized_fails() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_with_post_run(&temp_dir, "test-vault");

    // Start a run
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-start");

    let output = cmd.assert().success();
    let run_name = String::from_utf8(output.get_output().stdout.clone())
        .unwrap()
        .trim()
        .to_string();

    // End the run (first time - should succeed)
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-end")
        .arg(&run_name);

    cmd.assert().success();

    // End the run again (should fail)
    let mut cmd = cargo_bin_cmd!("capsula");
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run-end")
        .arg(&run_name);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("already been finalized"));
}
