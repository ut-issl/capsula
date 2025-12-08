#![expect(clippy::unwrap_used)]

use capsula_capture_git_repo::GitHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::fs;
use std::process::Command;
use ulid::Ulid;

#[test]
fn git_hook_captures_clean_repo() {
    // Arrange - create a temporary git repository
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("git init failed");

    // Configure git
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(temp_dir.join("test.txt"), b"initial").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: run_dir.clone(),
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert!(json.get("sha").is_some(), "Should capture commit SHA");
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(false),
        "Clean repo should not be dirty"
    );
    assert!(
        !captured.abort_requested(),
        "Clean repo should not request abort"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_captures_dirty_repo_with_allow_dirty() {
    // Arrange - create a git repository with uncommitted changes
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(temp_dir.join("test.txt"), b"initial").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Make uncommitted changes
    fs::write(temp_dir.join("test.txt"), b"modified").unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": true
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: run_dir.clone(),
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(true),
        "Repo with uncommitted changes should be dirty"
    );
    assert!(
        !captured.abort_requested(),
        "Should not abort when allow_dirty is true"
    );

    // Verify patch file was created
    let patch_file = run_dir.join("test-repo.patch");
    assert!(
        patch_file.exists(),
        "Patch file should be created for dirty repo"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_requests_abort_for_dirty_repo_when_not_allowed() {
    // Arrange
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(temp_dir.join("test.txt"), b"initial").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Make uncommitted changes
    fs::write(temp_dir.join("test.txt"), b"modified").unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: run_dir.clone(),
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert!(
        captured.abort_requested(),
        "Should request abort when dirty and allow_dirty is false"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_ignores_git_ignored_files() {
    // Arrange - create a git repository with ignored files (like .capsula directory)
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(temp_dir.join("test.txt"), b"initial").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create an ignored directory (simulating .capsula directory)
    let ignored_dir = temp_dir.join(".capsula");
    fs::create_dir_all(&ignored_dir).unwrap();
    fs::write(ignored_dir.join("data.json"), b"some data").unwrap();

    // Create .gitignore inside the ignored directory to make it ignored
    // (This simulates how Capsula creates .gitignore inside .capsula)
    fs::write(ignored_dir.join(".gitignore"), b"*\n").unwrap();

    // Verify git sees this as ignored
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&temp_dir)
        .output()
        .expect("git status failed");
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    assert!(
        status_str.trim().is_empty(),
        "Git status should be empty (ignored files don't count). Got: {status_str}",
    );

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: run_dir.clone(),
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(false),
        "Repo with only ignored files should not be dirty"
    );
    assert!(
        !captured.abort_requested(),
        "Should not abort when only ignored files are present"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_detects_untracked_files_as_dirty() {
    // Arrange - create a git repository with untracked (non-ignored) files
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(temp_dir.join("test.txt"), b"initial").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create an untracked file (not ignored)
    fs::write(temp_dir.join("untracked.txt"), b"new file").unwrap();

    // Verify git sees this as untracked
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&temp_dir)
        .output()
        .expect("git status failed");
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    assert!(
        status_str.contains("untracked.txt"),
        "Git status should show untracked file. Got: {status_str}",
    );

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: run_dir.clone(),
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(true),
        "Repo with untracked files should be dirty"
    );
    assert!(
        captured.abort_requested(),
        "Should abort when untracked files are present and allow_dirty is false"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}
