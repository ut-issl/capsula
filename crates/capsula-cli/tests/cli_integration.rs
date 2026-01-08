use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a minimal capsula.toml config
fn create_test_config(dir: &TempDir, vault_name: &str) -> PathBuf {
    let config_path = dir.path().join("capsula.toml");
    let config_content = format!(
        r#"
[vault]
name = "{}"

[[pre-run.hooks]]
id = "capture-cwd"
"#,
        vault_name
    );
    fs::write(&config_path, config_content).unwrap();
    config_path
}

#[test]
fn test_capsula_run_creates_run_directory() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = Command::cargo_bin("capsula").unwrap();
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
        .filter_map(|e| e.ok())
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
            .filter_map(|e| e.ok())
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
fn test_capsula_list_shows_runs() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    // First, create a run
    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert().success();

    // Now list runs
    let mut cmd = Command::cargo_bin("capsula").unwrap();
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
    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert().success();

    // Try to push without server URL (should fail)
    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("push")
        .arg("01234567890123456789012345"); // Some fake ID

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Server URL not specified"));
}

#[test]
fn test_capsula_vaults_list_requires_server_url() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    // Try to list vaults without server URL (should fail)
    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("vaults")
        .arg("list");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Server URL not specified"));
}

#[test]
fn test_capsula_run_with_nonexistent_config() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg("nonexistent.toml")
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Configuration file not found"));
}

#[test]
fn test_capsula_run_without_command() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir, "test-vault");

    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("No command specified"));
}

#[test]
fn test_capsula_config_with_server_url() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("capsula.toml");
    let config_content = r#"
[vault]
name = "test-vault"

server = "http://localhost:3000"

[[pre-run.hooks]]
id = "capture-cwd"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create a run to ensure config is valid
    let mut cmd = Command::cargo_bin("capsula").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("echo")
        .arg("test");

    cmd.assert().success();
}
