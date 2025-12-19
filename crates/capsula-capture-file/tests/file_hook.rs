#![expect(clippy::unwrap_used, reason = "unwrap is acceptable in test code")]

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
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture one file");

    let file_info = &files[0];
    assert!(file_info.get("path").is_some(), "Should have path");
    assert!(
        file_info.get("copied_path").is_some(),
        "Should have copied_path"
    );
    assert!(file_info.get("hash").is_some(), "Should have hash");

    // Verify file was actually copied
    let copied_path = run_dir.join("test.txt");
    assert!(copied_path.exists(), "File should be copied to run_dir");

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
    let data_dir = temp_dir.join("data");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();
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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture the file in subdirectory");

    let file_info = &files[0];
    assert!(file_info.get("path").is_some(), "Should have path");
    assert!(
        file_info.get("copied_path").is_some(),
        "Should have copied_path"
    );
    assert!(file_info.get("hash").is_some(), "Should have hash");

    // Verify file was copied
    let copied_path = run_dir.join("input.txt");
    assert!(copied_path.exists(), "File should be copied to run_dir");

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
    let nested_dir = temp_dir.join("data").join("deep").join("nested");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();
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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(
        files.len(),
        1,
        "Should capture the file in deeply nested subdirectory"
    );

    // Verify file was copied
    let copied_path = run_dir.join("config.json");
    assert!(
        copied_path.exists(),
        "File should be copied to run_dir with just the filename"
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn file_hook_wildcard_in_subdirectory() {
    // Arrange - create subdirectory with multiple files
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let data_dir = temp_dir.join("data");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();
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
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
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
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

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
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 1, "Should capture one file");

    // Verify file was moved
    let moved_path = run_dir.join("moveme.txt");
    assert!(moved_path.exists(), "File should exist in run_dir");
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
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

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
    let would_be_copied = run_dir.join("metadata.txt");
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
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&run_dir).unwrap();

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
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    let files = json.get("files").and_then(|v| v.as_array()).unwrap();
    assert_eq!(files.len(), 2, "Should capture only .log files");

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}
