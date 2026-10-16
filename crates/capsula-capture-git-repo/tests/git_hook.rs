//! Tests for the `GitHook` capturing functionality.
#![cfg(test)]

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
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init", "-b", "main"])
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
    Command::new("git")
        .args(["config", "commit.gpgSign", "false"])
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
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
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
    assert!(outcome.is_success(), "Clean repo should succeed");

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

/// Helper to initialize a git repo in a temp directory with one committed file.
/// Returns the temp directory path.
fn init_git_repo() -> std::path::PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::generate()));
    fs::create_dir_all(&temp_dir).unwrap();

    Command::new("git")
        .args(["init", "-b", "main"])
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
    let bare_dir = std::env::temp_dir().join(format!("capsula_git_bare_{}", Ulid::generate()));
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
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(true),
        "Pushed commit should be detected as pushed"
    );
    assert!(outcome.is_success(), "Should succeed when commit is pushed");

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
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(false),
        "Unpushed commit should be detected as not pushed"
    );
    assert!(
        outcome.is_success(),
        "Should succeed when require_pushed is false (default)"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
}

#[test]
fn git_hook_fails_on_unpushed_when_required() {
    // Arrange
    let temp_dir = init_git_repo();
    let bare_dir = add_bare_remote(&temp_dir, "origin");
    // Do NOT push

    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert_eq!(
        outcome.failure_reason(),
        Some("HEAD commit is not pushed to remote 'origin'"),
        "Should fail when require_pushed is true and commit is not pushed"
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
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "origin"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert - the old commit is still reachable via origin (ancestor of remote tip)
    assert_eq!(
        json.get("is_pushed").and_then(serde_json::Value::as_bool),
        Some(true),
        "Commit behind remote tip should still be detected as pushed (ancestor check)"
    );
    assert!(
        outcome.is_success(),
        "Should succeed for a pushed ancestor commit"
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
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    // Check against "upstream" remote
    let config = json!({
        "name": "test-repo",
        "path": ".",
        "require_pushed": true,
        "remote": "upstream"
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
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
    let artifact_dir2 = run_dir2.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir2).unwrap();
    let run_metadata2 = PreparedRun {
        id: Ulid::generate(),
        name: "test-run-2".to_string(),
        command: vec![],
        run_dir: run_dir2,
        project_root: temp_dir.clone(),
    };
    let params2 = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir2);

    let outcome_origin = hook_origin.run(&run_metadata2, &params2).expect("run ok");
    let json_origin = outcome_origin
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    assert_eq!(
        json_origin
            .get("is_pushed")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "Commit pushed to 'upstream' should NOT be detected when remote='origin'"
    );
    assert!(
        outcome_origin.is_failure(),
        "require_pushed should fail when the configured remote does not contain HEAD"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&bare_dir).ok();
}

#[test]
fn git_hook_captures_dirty_repo_with_allow_dirty() {
    // Arrange - create a git repository with uncommitted changes
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init", "-b", "main"])
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
    Command::new("git")
        .args(["config", "commit.gpgSign", "false"])
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
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(true),
        "Repo with uncommitted changes should be dirty"
    );
    assert!(
        outcome.is_success(),
        "Should succeed when allow_dirty is true"
    );

    // Verify patch file was created in artifact directory
    let patch_file = artifact_dir.join("test-repo.patch");
    assert!(
        patch_file.exists(),
        "Patch file should be created for dirty repo"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_fails_for_dirty_repo_when_not_allowed() {
    // Arrange
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init", "-b", "main"])
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
    Command::new("git")
        .args(["config", "commit.gpgSign", "false"])
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
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert_eq!(
        outcome.failure_reason(),
        Some("repository has uncommitted changes"),
        "Should fail when dirty and allow_dirty is false"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_ignores_git_ignored_files() {
    // Arrange - create a git repository with ignored files (like .capsula directory)
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init", "-b", "main"])
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
    Command::new("git")
        .args(["config", "commit.gpgSign", "false"])
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
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(false),
        "Repo with only ignored files should not be dirty"
    );
    assert!(
        outcome.is_success(),
        "Should succeed when only ignored files are present"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_tag_head_creates_lightweight_tag() {
    // Arrange
    let temp_dir = init_git_repo();
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false,
        "tag_head": true
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "chubby-back".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert - tag field in output
    assert_eq!(
        json.get("tag").and_then(serde_json::Value::as_str),
        Some("capsula/chubby-back"),
        "Output should contain the tag name"
    );

    // Assert - tag exists in the repository
    let tag_output = Command::new("git")
        .args(["tag", "-l", "capsula/chubby-back"])
        .current_dir(&temp_dir)
        .output()
        .expect("git tag failed");
    let tag_str = String::from_utf8_lossy(&tag_output.stdout);
    assert!(
        tag_str.trim() == "capsula/chubby-back",
        "Lightweight tag 'capsula/chubby-back' should exist in the repository. Got: {tag_str}"
    );

    // Assert - tag points to the correct commit
    let sha = json.get("sha").and_then(serde_json::Value::as_str).unwrap();
    let tag_rev_output = Command::new("git")
        .args(["rev-parse", "capsula/chubby-back"])
        .current_dir(&temp_dir)
        .output()
        .expect("git rev-parse failed");
    let tag_sha = String::from_utf8_lossy(&tag_rev_output.stdout);
    assert_eq!(
        tag_sha.trim(),
        sha,
        "Tag should point to the captured HEAD commit"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_no_tag_when_tag_head_is_false() {
    // Arrange
    let temp_dir = init_git_repo();
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false,
        "tag_head": false
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "quiet-hill".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert - tag field is null in output
    assert!(
        json.get("tag").unwrap().is_null(),
        "Tag field should be null when tag_head is false"
    );

    // Assert - no tag in the repository
    let tag_output = Command::new("git")
        .args(["tag", "-l", "capsula/*"])
        .current_dir(&temp_dir)
        .output()
        .expect("git tag failed");
    let tag_str = String::from_utf8_lossy(&tag_output.stdout);
    assert!(
        tag_str.trim().is_empty(),
        "No capsula tags should exist when tag_head is false. Got: {tag_str}"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_detects_untracked_files_as_dirty() {
    // Arrange - create a git repository with untracked (non-ignored) files
    let temp_dir = std::env::temp_dir().join(format!("capsula_git_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-git-repo");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init", "-b", "main"])
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
    Command::new("git")
        .args(["config", "commit.gpgSign", "false"])
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
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("is_dirty").and_then(serde_json::Value::as_bool),
        Some(true),
        "Repo with untracked files should be dirty"
    );
    assert!(
        outcome.is_failure(),
        "Should fail when untracked files are present and allow_dirty is false"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn git_hook_rejects_unknown_config_fields() {
    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty_typo": false
    });

    let result = <GitHook as Hook<PreRun>>::from_config(&config, &std::env::temp_dir());

    assert!(result.is_err(), "unknown config fields should be rejected");
}

#[test]
fn git_hook_resolves_path_when_run() {
    // Building the hook must not depend on the file system, so a missing path
    // is an error of the run, not of the configuration.
    let config = json!({
        "name": "test-repo",
        "path": format!("capsula-missing-{}", Ulid::generate()),
    });
    let hook = <GitHook as Hook<PreRun>>::from_config(&config, &std::env::temp_dir())
        .expect("a missing path should not fail the configuration");
    let run = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: std::env::temp_dir(),
        project_root: std::env::temp_dir(),
    };

    let err = <GitHook as Hook<PreRun>>::run(&hook, &run, &RuntimeParams::default())
        .expect_err("running with a missing path should fail");
    let message = std::error::Error::source(&err).map(ToString::to_string);
    assert!(
        message
            .as_deref()
            .is_some_and(|m| m.contains("Failed to resolve repository path")),
        "unexpected error: {message:?}"
    );
}

/// Run `capture-git-repo` with `tag_head = true` on a dirty repository with
/// `allow_dirty = false`, and return the captured tag and the tags in the repo.
fn run_tag_head_on_dirty_repo<P>() -> (Option<String>, String)
where
    P: capsula_core::hook::PhaseMarker + std::fmt::Debug,
    GitHook: Hook<P>,
{
    let temp_dir = init_git_repo();
    fs::write(temp_dir.join("test.txt"), b"modified").unwrap();
    let artifact_dir = temp_dir.join("run").join("artifact");
    fs::create_dir_all(&artifact_dir).unwrap();

    let config = json!({
        "name": "test-repo",
        "path": ".",
        "allow_dirty": false,
        "tag_head": true
    });
    let hook = <GitHook as Hook<P>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "dirty-run".to_string(),
        command: vec![],
        run_dir: temp_dir.join("run"),
        project_root: temp_dir.clone(),
    };

    let outcome = <GitHook as Hook<P>>::run(
        &hook,
        &run_metadata,
        &RuntimeParams::<P>::with_artifact_dir(artifact_dir),
    )
    .expect("run ok");
    assert!(outcome.is_failure(), "a dirty repository should fail");
    let json = outcome.output().serialize_json().expect("serialize ok");
    let tag = json
        .get("tag")
        .and_then(serde_json::Value::as_str)
        .map(String::from);

    let tags = Command::new("git")
        .args(["tag", "-l", "capsula/*"])
        .current_dir(&temp_dir)
        .output()
        .expect("git tag failed");
    let tags = String::from_utf8_lossy(&tags.stdout).trim().to_string();

    fs::remove_dir_all(&temp_dir).ok();
    (tag, tags)
}

#[test]
fn git_hook_tag_head_skips_tag_when_pre_run_hook_fails() {
    // The failure stops the run, so there is no run whose commit to keep.
    let (tag, tags) = run_tag_head_on_dirty_repo::<PreRun>();

    assert_eq!(tag, None);
    assert_eq!(tags, "", "no tag should be created");
}

#[test]
fn git_hook_tag_head_tags_commit_when_post_run_hook_fails() {
    // The command has already run, so its commit is still worth keeping.
    let (tag, tags) = run_tag_head_on_dirty_repo::<capsula_core::hook::PostRun>();

    assert_eq!(tag.as_deref(), Some("capsula/dirty-run"));
    assert_eq!(tags, "capsula/dirty-run");
}
