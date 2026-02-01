//! Tests for the `GitHook` capturing functionality.
#![expect(clippy::unwrap_used, reason = "unwrap is acceptable in test code")]

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
        run_dir,
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
    assert_eq!(
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(false),
        "Repo with no remote should not be pushed"
    );
    assert!(
        !captured.abort_requested(),
        "Clean repo should not request abort"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

/// Helper to initialize a git repo in a temp directory with one committed file.
/// Returns the temp directory path.
fn init_git_repo() -> std::path::PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::new()));
    fs::create_dir_all(&temp_dir).unwrap();

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
    // Disable signing to avoid GPG prompts in tests
    Command::new("git")
        .args(["config", "commit.gpgSign", "false"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "tag.gpgSign", "false"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

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

    temp_dir
}

/// Helper to create a bare remote repo and add it as a remote to the working repo.
/// Returns the bare repo path.
fn add_bare_remote(working_dir: &std::path::Path, remote_name: &str) -> std::path::PathBuf {
    let bare_dir = std::env::temp_dir().join(format!("capsula_git_bare_{}", Ulid::new()));
    Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare_dir)
        .output()
        .expect("git init --bare failed");

    Command::new("git")
        .args(["remote", "add", remote_name])
        .arg(&bare_dir)
        .current_dir(working_dir)
        .output()
        .unwrap();

    bare_dir
}

/// Helper to push the current branch to a remote.
fn push_to_remote(working_dir: &std::path::Path, remote_name: &str) {
    let output = Command::new("git")
        .args(["push", remote_name, "HEAD"])
        .current_dir(working_dir)
        .output()
        .expect("git push failed");
    assert!(
        output.status.success(),
        "git push failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn git_hook_detects_pushed_commit() {
    // Arrange
    let temp_dir = init_git_repo();
    let bare_dir = add_bare_remote(&temp_dir, "origin");
    push_to_remote(&temp_dir, "origin");

    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
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
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(true),
        "Pushed commit should be detected as pushed"
    );
    assert!(
        !captured.abort_requested(),
        "Should not abort when commit is pushed"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
}

#[test]
fn git_hook_detects_unpushed_commit() {
    // Arrange - repo with remote but commit not pushed
    let temp_dir = init_git_repo();
    let bare_dir = add_bare_remote(&temp_dir, "origin");
    // Do NOT push

    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
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
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(false),
        "Unpushed commit should be detected as not pushed"
    );
    assert!(
        !captured.abort_requested(),
        "Should not abort when require_pushed is false (default)"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
}

#[test]
fn git_hook_aborts_on_unpushed_when_required() {
    // Arrange
    let temp_dir = init_git_repo();
    let bare_dir = add_bare_remote(&temp_dir, "origin");
    // Do NOT push

    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert!(
        captured.abort_requested(),
        "Should abort when require_pushed is true and commit is not pushed"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
}

#[test]
fn git_hook_pushed_commit_behind_remote() {
    // Arrange - push, then make another commit locally so HEAD~1 is behind remote tip
    let temp_dir = init_git_repo();
    let bare_dir = add_bare_remote(&temp_dir, "origin");
    push_to_remote(&temp_dir, "origin");

    // Make another commit and push it, so the original commit is behind
    fs::write(temp_dir.join("test.txt"), b"second").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Second commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    push_to_remote(&temp_dir, "origin");

    // Checkout the first commit (detached HEAD)
    Command::new("git")
        .args(["checkout", "HEAD~1"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&run_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert - the old commit is still reachable via origin (ancestor of remote tip)
    assert_eq!(
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(true),
        "Commit behind remote tip should still be detected as pushed (ancestor check)"
    );
    assert!(
        !captured.abort_requested(),
        "Should not abort for a pushed ancestor commit"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
}

#[test]
fn git_hook_custom_remote() {
    // Arrange - push to a remote named "upstream" instead of "origin"
    let temp_dir = init_git_repo();
    let bare_dir = add_bare_remote(&temp_dir, "upstream");
    push_to_remote(&temp_dir, "upstream");

    let run_dir = temp_dir.join("run");
    fs::create_dir_all(&run_dir).unwrap();

    // Check against "upstream" remote
    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "upstream"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert - pushed to "upstream", checking "upstream" → found
    assert_eq!(
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(true),
        "Commit pushed to 'upstream' should be detected when remote='upstream'"
    );

    // Now check against default "origin" remote - should NOT be found
    let config_origin = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook_origin =
        <GitHook as Hook<PreRun>>::from_config(&config_origin, &temp_dir).expect("from_config ok");

    let run_dir2 = temp_dir.join("run2");
    fs::create_dir_all(&run_dir2).unwrap();
    let run_metadata2 = PreparedRun {
        id: Ulid::new(),
        name: "test-run-2".to_string(),
        command: vec![],
        run_dir: run_dir2,
        project_root: temp_dir.clone(),
    };

    let captured_origin = hook_origin.run(&run_metadata2, &params).expect("run ok");
    let json_origin = captured_origin
        .serialize_json()
        .expect("serialization should succeed");

    assert_eq!(
        json_origin
            .get("is_pushed")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "Commit pushed to 'upstream' should NOT be detected when remote='origin'"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
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
        run_dir,
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
        run_dir,
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
        run_dir,
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
