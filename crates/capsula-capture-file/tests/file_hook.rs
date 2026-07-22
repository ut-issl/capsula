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
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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
    let copied_path = artifact_dir.join("data").join("input.txt");
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
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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
    let copied_path = artifact_dir
        .join("data")
        .join("deep")
        .join("nested")
        .join("config.json");
    assert!(
        copied_path.exists(),
        "File should be copied to artifact dir"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_preserves_paths_for_files_with_the_same_name() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let first_dir = temp_dir.join("a");
    let second_dir = temp_dir.join("b");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&first_dir).unwrap();
    fs::create_dir_all(&second_dir).unwrap();
    fs::write(first_dir.join("config.json"), b"first").unwrap();
    fs::write(second_dir.join("config.json"), b"second").unwrap();

    let config = json!({
        "glob": "**/config.json",
        "mode": "copy",
        "hash": "none"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");
    let files = json
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(
        fs::read(artifact_dir.join("a/config.json")).unwrap(),
        b"first"
    );
    assert_eq!(
        fs::read(artifact_dir.join("b/config.json")).unwrap(),
        b"second"
    );
    assert_ne!(files[0].get("copied_path"), files[1].get("copied_path"));

    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn file_hook_wildcard_in_subdirectory() {
    // Arrange - create subdirectory with multiple files
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
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
        id: Ulid::new(),
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

#[test]
fn file_hook_rejects_absolute_glob() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    fs::create_dir_all(&temp_dir).unwrap();
    let absolute_glob = temp_dir.join("*.txt").to_string_lossy().into_owned();
    let config = json!({
        "glob": absolute_glob,
        "mode": "move",
        "hash": "none"
    });

    let result = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir);

    assert!(result.is_err(), "absolute globs should be rejected");
    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn file_hook_rejects_parent_traversal_glob() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    fs::create_dir_all(&temp_dir).unwrap();
    let config = json!({
        "glob": "../outside/*.txt",
        "mode": "move",
        "hash": "none"
    });

    let result = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir);

    assert!(result.is_err(), "parent traversal should be rejected");
    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn file_hook_recursive_glob_preserves_double_star_semantics() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let nested_dir = temp_dir.join("nested").join("deep");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(temp_dir.join("root.txt"), b"root").unwrap();
    fs::write(nested_dir.join("nested.txt"), b"nested").unwrap();

    let config = json!({
        "glob": "**/*.txt",
        "mode": "none",
        "hash": "none"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");
    let files = json
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();

    assert_eq!(files.len(), 2, "double-star glob should match recursively");
    fs::remove_dir_all(temp_dir).ok();
}

#[cfg(unix)]
#[test]
fn file_hook_rejects_symlink_before_moving_any_matches() {
    use std::os::unix::fs::symlink;

    let parent = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let project_root = parent.join("project");
    let run_dir = project_root.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let safe_file = project_root.join("a-safe.txt");
    let outside_file = parent.join("outside-secret.txt");
    let symlink_path = project_root.join("z-secret.txt");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::write(&safe_file, b"safe").unwrap();
    fs::write(&outside_file, b"secret").unwrap();
    symlink(&outside_file, &symlink_path).unwrap();

    let config = json!({
        "glob": "*.txt",
        "mode": "move",
        "hash": "sha256"
    });
    let hook =
        <FileHook as Hook<PreRun>>::from_config(&config, &project_root).expect("from_config ok");
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root,
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    let result = hook.run(&run_metadata, &params);

    assert!(result.is_err(), "a matching symlink should fail the hook");
    assert!(safe_file.exists(), "safe files must not be partially moved");
    assert!(
        outside_file.exists(),
        "the symlink target must remain untouched"
    );
    assert!(symlink_path.exists(), "the symlink must remain untouched");
    assert!(
        fs::read_dir(&artifact_dir).unwrap().next().is_none(),
        "the artifact directory must remain empty"
    );
    fs::remove_dir_all(parent).ok();
}

#[cfg(unix)]
#[test]
fn file_hook_does_not_descend_into_symlinked_directory() {
    use std::os::unix::fs::symlink;

    let parent = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let project_root = parent.join("project");
    let outside_dir = parent.join("outside");
    let outside_file = outside_dir.join("secret.txt");
    let symlink_path = project_root.join("external");
    let run_dir = project_root.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();
    fs::write(&outside_file, b"secret").unwrap();
    symlink(&outside_dir, &symlink_path).unwrap();

    let config = json!({
        "glob": "external/*.txt",
        "mode": "move",
        "hash": "sha256"
    });
    let hook =
        <FileHook as Hook<PreRun>>::from_config(&config, &project_root).expect("from_config ok");
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root,
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");
    let files = json
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();

    assert!(files.is_empty(), "files behind symlinks must not match");
    assert!(
        outside_file.exists(),
        "the outside file must remain untouched"
    );
    assert!(symlink_path.exists(), "the symlink must remain untouched");
    assert!(
        fs::read_dir(&artifact_dir).unwrap().next().is_none(),
        "the artifact directory must remain empty"
    );
    fs::remove_dir_all(parent).ok();
}

#[cfg(unix)]
#[test]
fn file_hook_preserves_backslashes_in_unix_globs() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-file");
    let file_name = r"report\2026.txt";
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::write(temp_dir.join(file_name), b"report").unwrap();

    let config = json!({
        "glob": file_name,
        "mode": "copy",
        "hash": "none"
    });
    let hook = <FileHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir,
        project_root: temp_dir.clone(),
    };
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");
    let files = json
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();

    assert_eq!(files.len(), 1, "the backslash should remain literal");
    assert!(
        artifact_dir.join(file_name).exists(),
        "the backslash-named file should be copied"
    );
    fs::remove_dir_all(temp_dir).ok();
}
