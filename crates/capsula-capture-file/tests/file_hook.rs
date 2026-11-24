//! Tests for the `FileHook` capturing functionality.
#![cfg(test)]

use capsula_capture_file::FileHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::fs;
use ulid::Ulid;

#[test]
fn file_hook_captures_files_with_copy_mode() {
    // Arrange - create a temporary directory and file
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    let test_file = temp_dir.join("test.txt");
    fs::write(&test_file, b"test content").unwrap();

    let config = json!({
        "glob": "*.txt",
        "mode": "copy",
        "hash": "sha256"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture one file");

    let file_info = &files[0];
    assert!(file_info.get("path").is_some(), "Should have path");
    assert!(
        file_info.get("copied_path").is_some(),
        "Should have copied_path"
    );
    assert!(file_info.get("hash").is_some(), "Should have hash");

    // Verify file was actually copied (preserving relative path)
    let copied_path = artifact_dir.join("test.txt");
    assert!(
        copied_path.exists(),
        "File should be copied to artifact dir"
    );

    // Verify original file still exists
    assert!(
        test_file.exists(),
        "Original file should still exist in copy mode"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_captures_files_in_subdirectories() {
    // Arrange - create a subdirectory structure
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let data_dir = temp_dir.join("data");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let test_file = data_dir.join("input.txt");
    fs::write(&test_file, b"subdirectory content").unwrap();

    let config = json!({
        "glob": "data/input.txt",
        "mode": "copy",
        "hash": "sha256"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture the file in subdirectory");

    let file_info = &files[0];
    assert!(file_info.get("path").is_some(), "Should have path");
    assert!(
        file_info.get("copied_path").is_some(),
        "Should have copied_path"
    );
    assert!(file_info.get("hash").is_some(), "Should have hash");

    // Verify the project-relative path was preserved
    let copied_path = artifact_dir.join("data/input.txt");
    assert!(
        copied_path.exists(),
        "File should be copied to artifact dir"
    );

    // Verify original file still exists
    assert!(
        test_file.exists(),
        "Original file should still exist in subdirectory"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_captures_files_in_nested_subdirectories() {
    // Arrange - create a deeply nested directory structure
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let nested_dir = temp_dir.join("data").join("deep").join("nested");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();

    let test_file = nested_dir.join("config.json");
    fs::write(&test_file, b"{\"key\": \"value\"}").unwrap();

    let config = json!({
        "glob": "data/deep/nested/config.json",
        "mode": "copy",
        "hash": "sha256"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(
        files.len(),
        1,
        "Should capture the file in deeply nested subdirectory"
    );

    // Verify the project-relative path was preserved
    let copied_path = artifact_dir.join("data/deep/nested/config.json");
    assert!(
        copied_path.exists(),
        "File should be copied to artifact dir"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_wildcard_in_subdirectory() {
    // Arrange - create subdirectory with multiple files
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let data_dir = temp_dir.join("data");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    fs::write(data_dir.join("file1.txt"), b"content1").unwrap();
    fs::write(data_dir.join("file2.txt"), b"content2").unwrap();
    fs::write(data_dir.join("other.log"), b"log").unwrap();

    let config = json!({
        "glob": "data/*.txt",
        "mode": "none",
        "hash": "none"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(
        files.len(),
        2,
        "Should capture only .txt files in subdirectory"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_captures_files_with_move_mode() {
    // Arrange
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    let test_file = temp_dir.join("moveme.txt");
    fs::write(&test_file, b"move test").unwrap();

    let config = json!({
        "glob": "moveme.txt",
        "mode": "move",
        "hash": "none"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture one file");

    // Verify file was moved
    let moved_path = artifact_dir.join("moveme.txt");
    assert!(moved_path.exists(), "File should exist in artifact dir");
    assert!(
        !test_file.exists(),
        "Original file should not exist in move mode"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_captures_files_with_none_mode() {
    // Arrange
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    let test_file = temp_dir.join("metadata.txt");
    fs::write(&test_file, b"just metadata").unwrap();

    let config = json!({
        "glob": "metadata.txt",
        "mode": "none",
        "hash": "sha256"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture one file");

    let file_info = &files[0];
    assert!(file_info.get("path").is_some(), "Should have path");
    assert!(
        file_info.get("copied_path").unwrap().is_null(),
        "Should not have copied_path in none mode"
    );
    assert!(file_info.get("hash").is_some(), "Should have hash");

    // Verify file was not copied
    let would_be_copied = artifact_dir.join("metadata.txt");
    assert!(
        !would_be_copied.exists(),
        "File should not be copied in none mode"
    );

    // Verify original file still exists
    assert!(test_file.exists(), "Original file should still exist");

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_matches_glob_pattern() {
    // Arrange - create multiple files
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::generate()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&artifact_dir).unwrap();

    fs::write(temp_dir.join("file1.log"), b"log1").unwrap();
    fs::write(temp_dir.join("file2.log"), b"log2").unwrap();
    fs::write(temp_dir.join("other.txt"), b"other").unwrap();

    let config = json!({
        "glob": "*.log",
        "mode": "none",
        "hash": "none"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 2, "Should capture only .log files");

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_rejects_unknown_config_fields() {
    let config = json!({
        "glob": "*.txt",
        "mode": "copy",
        "copy": true
    });

    let result = <FileHook as Hook<PreRun>>::from_config(&config, &std::env::temp_dir());

    assert!(result.is_err(), "unknown config fields should be rejected");
}

/// A temporary project with a run directory and a `capture-file` artifact
/// directory, removed when dropped.
struct Project {
    root: std::path::PathBuf,
    artifact_dir: std::path::PathBuf,
}

impl Project {
    fn new() -> Self {
        Self::named("project")
    }

    /// A project whose root directory has the given name.
    fn named(name: &str) -> Self {
        let root = std::env::temp_dir()
            .join(format!("capsula_test_{}", Ulid::generate()))
            .join(name);
        let artifact_dir = root.join("run").join("pre-0-capture-file");
        fs::create_dir_all(&artifact_dir).unwrap();
        Self { root, artifact_dir }
    }

    /// A path next to the project root, outside the project.
    fn outside(&self, name: &str) -> std::path::PathBuf {
        self.root.parent().unwrap().join(name)
    }

    fn run(
        &self,
        config: &serde_json::Value,
    ) -> capsula_core::error::CapsulaResult<serde_json::Map<String, serde_json::Value>> {
        let hook = <FileHook as Hook<PreRun>>::from_config(config, &self.root)?;
        let run_metadata = PreparedRun {
            id: Ulid::generate(),
            name: "test-run".to_string(),
            command: vec![],
            run_dir: self.root.join("run"),
            project_root: self.root.clone(),
        };
        let params = RuntimeParams::<PreRun>::with_artifact_dir(self.artifact_dir.clone());
        let outcome = hook.run(&run_metadata, &params)?;
        Ok(outcome.output().serialize_json().unwrap())
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(self.root.parent().unwrap()).ok();
    }
}

fn captured_paths(json: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    json["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn file_hook_rejects_absolute_glob_at_build_time() {
    let project = Project::new();
    let secret = project.outside("secret.txt");
    fs::write(&secret, b"secret").unwrap();

    let result = <FileHook as Hook<PreRun>>::from_config(
        &json!({ "glob": secret.to_string_lossy(), "mode": "move" }),
        &project.root,
    );

    assert!(result.is_err(), "absolute globs should be rejected");
    assert!(secret.exists());
}

#[test]
fn file_hook_rejects_parent_traversal_glob_at_build_time() {
    let project = Project::new();

    let result = <FileHook as Hook<PreRun>>::from_config(
        &json!({ "glob": "../*.txt", "mode": "move" }),
        &project.root,
    );

    assert!(result.is_err(), "'..' globs should be rejected");
}

#[test]
fn file_hook_accepts_current_dir_prefix() {
    let project = Project::new();
    fs::write(project.root.join("a.txt"), b"a").unwrap();

    let json = project
        .run(&json!({ "glob": "./*.txt", "mode": "none" }))
        .expect("run ok");

    assert_eq!(captured_paths(&json).len(), 1);
}

#[test]
fn file_hook_double_star_matches_at_any_depth() {
    let project = Project::new();
    fs::create_dir_all(project.root.join("x/y")).unwrap();
    fs::write(project.root.join("top.txt"), b"top").unwrap();
    fs::write(project.root.join("x/y/deep.txt"), b"deep").unwrap();

    let json = project
        .run(&json!({ "glob": "**/*.txt", "mode": "none" }))
        .expect("run ok");

    assert_eq!(captured_paths(&json).len(), 2);
}

#[test]
fn file_hook_matches_glob_characters_in_the_project_root_literally() {
    let project = Project::named("run[1]");
    fs::write(project.root.join("a.txt"), b"a").unwrap();

    let json = project
        .run(&json!({ "glob": "*.txt", "mode": "none" }))
        .expect("run ok");

    assert_eq!(captured_paths(&json).len(), 1);
}

#[cfg(unix)]
#[test]
fn file_hook_follows_a_symlinked_directory() {
    let project = Project::new();
    let storage = project.outside("storage");
    fs::create_dir_all(&storage).unwrap();
    fs::write(storage.join("input.csv"), b"input").unwrap();
    std::os::unix::fs::symlink(&storage, project.root.join("data")).unwrap();

    let json = project
        .run(&json!({ "glob": "data/*.csv", "mode": "copy" }))
        .expect("run ok");

    assert_eq!(captured_paths(&json).len(), 1);
    assert!(storage.join("input.csv").exists());
}

#[cfg(unix)]
#[test]
fn file_hook_ignores_unreadable_directory_the_glob_cannot_reach() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    let results = project.root.join("results");
    let private = project.root.join("private");
    fs::create_dir_all(&results).unwrap();
    fs::create_dir_all(private.join("inner")).unwrap();
    fs::write(results.join("a.csv"), b"a").unwrap();
    fs::set_permissions(&private, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&private).is_ok() {
        // Running with privileges that bypass permissions; nothing to test.
        fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let result = project.run(&json!({ "glob": "results/*.csv", "mode": "none" }));
    fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();

    let json = result.expect("an unrelated unreadable directory should not fail the hook");
    assert_eq!(captured_paths(&json).len(), 1);
}

#[cfg(unix)]
#[test]
fn file_hook_fails_on_unreadable_directory_the_glob_must_search() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    let private = project.root.join("private");
    fs::create_dir_all(&private).unwrap();
    fs::write(project.root.join("a.csv"), b"a").unwrap();
    fs::set_permissions(&private, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&private).is_ok() {
        // Running with privileges that bypass permissions; nothing to test.
        fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let result = project.run(&json!({ "glob": "**/*.csv", "mode": "none" }));
    fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        result.is_err(),
        "a directory that can contain a match but cannot be read should fail the hook"
    );
}

#[cfg(unix)]
#[test]
fn file_hook_treats_backslash_as_literal_on_unix() {
    let project = Project::new();
    let file_name = r"report\2026.txt";
    fs::write(project.root.join(file_name), b"report").unwrap();

    let json = project
        .run(&json!({ "glob": file_name, "mode": "copy", "hash": "none" }))
        .expect("run ok");

    assert_eq!(captured_paths(&json).len(), 1);
    assert!(project.artifact_dir.join(file_name).exists());
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!(
        "sha256:{}",
        capsula_core::util::hex_encode(sha2::Sha256::digest(bytes).as_ref())
    )
}

#[test]
fn file_hook_keeps_files_with_the_same_name_apart() {
    let project = Project::new();
    fs::create_dir_all(project.root.join("a")).unwrap();
    fs::create_dir_all(project.root.join("b")).unwrap();
    fs::write(project.root.join("a/config.json"), b"first").unwrap();
    fs::write(project.root.join("b/config.json"), b"second").unwrap();

    let json = project
        .run(&json!({ "glob": "**/config.json", "mode": "copy" }))
        .expect("run ok");

    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    for (file, relative) in [(&files[0], "a/config.json"), (&files[1], "b/config.json")] {
        let copied_path = project.artifact_dir.join(relative);
        assert_eq!(file["copied_path"], json!(copied_path));
        assert_eq!(
            file["hash"],
            json!(sha256(&fs::read(&copied_path).unwrap()))
        );
    }
    assert_eq!(
        fs::read(project.artifact_dir.join("a/config.json")).unwrap(),
        b"first"
    );
    assert_eq!(
        fs::read(project.artifact_dir.join("b/config.json")).unwrap(),
        b"second"
    );
}

#[test]
fn file_hook_never_overwrites_an_existing_artifact() {
    let project = Project::new();
    let source = project.root.join("data.txt");
    let existing = project.artifact_dir.join("data.txt");
    fs::write(&source, b"new").unwrap();
    fs::write(&existing, b"existing").unwrap();

    let result = project.run(&json!({ "glob": "*.txt", "mode": "move" }));

    let error = format!(
        "{:?}",
        result.expect_err("an existing artifact should fail the hook")
    );
    assert!(error.contains("ArtifactExists"), "{error}");
    assert_eq!(fs::read(&existing).unwrap(), b"existing");
    assert!(source.exists(), "the source must not be removed");
}

#[cfg(unix)]
#[test]
fn file_hook_move_archives_an_independent_copy() {
    let project = Project::new();
    let source = project.root.join("out.txt");
    let other_link = project.root.join("writer.dat");
    fs::write(&source, b"captured").unwrap();
    // A second name for the same inode stands in for a writer that still has
    // the file open after it is moved.
    fs::hard_link(&source, &other_link).unwrap();

    let json = project
        .run(&json!({ "glob": "*.txt", "mode": "move" }))
        .expect("run ok");
    fs::write(&other_link, b"modified after capture").unwrap();

    let archived = fs::read(project.artifact_dir.join("out.txt")).unwrap();
    assert_eq!(archived, b"captured");
    assert_eq!(json["files"][0]["hash"], json!(sha256(&archived)));
    assert!(!source.exists(), "the source should be removed");
}

#[cfg(unix)]
#[test]
fn file_hook_copy_preserves_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    let source = project.root.join("run.sh");
    fs::write(
        &source,
        b"#!/bin/sh
",
    )
    .unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o750)).unwrap();

    project
        .run(&json!({ "glob": "*.sh", "mode": "copy", "hash": "none" }))
        .expect("run ok");

    let mode = fs::metadata(project.artifact_dir.join("run.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o750);
}

#[test]
fn file_hook_checks_every_destination_before_moving_any_file() {
    let project = Project::new();
    fs::create_dir_all(project.root.join("a")).unwrap();
    fs::create_dir_all(project.root.join("b")).unwrap();
    fs::create_dir_all(project.artifact_dir.join("b")).unwrap();
    fs::write(project.root.join("a/data.txt"), b"a").unwrap();
    fs::write(project.root.join("b/data.txt"), b"b").unwrap();
    fs::write(project.artifact_dir.join("b/data.txt"), b"existing").unwrap();

    let result = project.run(&json!({ "glob": "*/data.txt", "mode": "move" }));

    let error = format!(
        "{:?}",
        result.expect_err("an existing artifact should fail the hook")
    );
    assert!(error.contains("ArtifactExists"), "{error}");
    assert!(
        project.root.join("a/data.txt").exists(),
        "a must not be moved"
    );
    assert!(!project.artifact_dir.join("a/data.txt").exists());
}

#[cfg(unix)]
#[test]
fn file_hook_records_files_moved_before_a_later_failure() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    fs::write(project.root.join("a.txt"), b"a").unwrap();
    fs::write(project.root.join("b.txt"), b"b").unwrap();
    let unreadable = project.root.join("b.txt");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&unreadable).is_ok() {
        // Running with privileges that bypass permissions; nothing to test.
        return;
    }

    let hook = <FileHook as Hook<PreRun>>::from_config(
        &json!({ "glob": "*.txt", "mode": "move" }),
        &project.root,
    )
    .unwrap();
    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: project.root.join("run"),
        project_root: project.root.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(project.artifact_dir.clone());
    let outcome = hook
        .run(&run_metadata, &params)
        .expect("the hook should produce output");
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(
        !outcome.is_success(),
        "the failed capture should fail the hook"
    );
    let json = outcome.output().serialize_json().unwrap();
    let paths = captured_paths(&json);
    assert_eq!(paths.len(), 1, "the moved file should stay recorded");
    assert!(paths[0].ends_with("a.txt"));
    assert!(!project.root.join("a.txt").exists());
    assert!(project.artifact_dir.join("a.txt").exists());
    assert!(unreadable.exists(), "the failed source must stay in place");
}

#[cfg(unix)]
#[test]
fn file_hook_moves_a_symlink_by_removing_only_the_link() {
    let project = Project::new();
    let target = project.outside("target.txt");
    let link = project.root.join("link.txt");
    fs::write(&target, b"target").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let json = project
        .run(&json!({ "glob": "*.txt", "mode": "move" }))
        .expect("run ok");

    let archived = fs::read(project.artifact_dir.join("link.txt")).unwrap();
    assert_eq!(archived, b"target");
    assert_eq!(json["files"][0]["hash"], json!(sha256(&archived)));
    assert!(
        link.symlink_metadata().is_err(),
        "the link should be removed"
    );
    assert!(target.exists(), "the link target should be kept");
}
