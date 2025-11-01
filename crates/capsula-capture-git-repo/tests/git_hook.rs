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
        json.get("is_dirty").and_then(|v| v.as_bool()),
        Some(false),
        "Clean repo should not be dirty"
    );
    assert!(!captured.abort_requested(), "Clean repo should not request abort");

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
        json.get("is_dirty").and_then(|v| v.as_bool()),
        Some(true),
        "Repo with uncommitted changes should be dirty"
    );
    assert!(!captured.abort_requested(), "Should not abort when allow_dirty is true");

    // Verify patch file was created
    let patch_file = run_dir.join("test-repo.patch");
    assert!(patch_file.exists(), "Patch file should be created for dirty repo");

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
    assert_eq!(
        captured.abort_requested(),
        true,
        "Should request abort when dirty and allow_dirty is false"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}
