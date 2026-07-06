//! Tests for the `DirHook` directory capture functionality.
#![cfg(test)]

use capsula_capture_dir::DirHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::fs;
use std::path::Path;
use ulid::Ulid;

fn test_run(project_root: &Path, run_dir: &Path) -> PreparedRun {
    PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: run_dir.to_path_buf(),
        project_root: project_root.to_path_buf(),
    }
}

#[test]
fn dir_hook_copies_directory_contents_preserving_tree() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-dir");
    let input_dir = temp_dir.join("input");
    let nested_dir = input_dir.join("nested");
    let empty_dir = input_dir.join("empty");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();
    fs::create_dir_all(&empty_dir).unwrap();
    fs::write(input_dir.join("root.txt"), b"root content").unwrap();
    fs::write(nested_dir.join("data.txt"), b"nested content").unwrap();

    let config = json!({
        "path": "input",
        "mode": "copy",
        "hash": "sha256"
    });
    let hook = <DirHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = test_run(&temp_dir, &run_dir);
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let output = captured
        .serialize_json()
        .expect("serialization should succeed");

    assert_eq!(
        output.get("captured_path").and_then(|value| value.as_str()),
        Some(artifact_dir.join("input").to_string_lossy().as_ref())
    );

    let directories = output
        .get("directories")
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(directories.len(), 2, "Should report nested and empty dirs");

    let files = output
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(files.len(), 2, "Should capture both files in the tree");
    assert!(
        files.iter().all(|file| file
            .get("hash")
            .and_then(|value| value.as_str())
            .is_some_and(|hash| hash.starts_with("sha256:"))),
        "Should hash every captured file"
    );

    assert!(artifact_dir.join("input/root.txt").exists());
    assert!(artifact_dir.join("input/nested/data.txt").exists());
    assert!(artifact_dir.join("input/empty").is_dir());
    assert!(input_dir.join("root.txt").exists());
    assert!(nested_dir.join("data.txt").exists());

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn dir_hook_hashes_directory_without_artifact_dir_in_none_mode() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let input_dir = temp_dir.join("input");
    fs::create_dir_all(&run_dir).unwrap();
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("metadata.txt"), b"metadata only").unwrap();

    let config = json!({
        "path": "input",
        "mode": "none",
        "hash": "sha256"
    });
    let hook = <DirHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = test_run(&temp_dir, &run_dir);
    let params = RuntimeParams::<PreRun>::default();

    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let output = captured
        .serialize_json()
        .expect("serialization should succeed");

    assert!(
        output.get("captured_path").unwrap().is_null(),
        "Should not need a destination in none mode"
    );
    let files = output
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(files.len(), 1, "Should still report files");
    assert!(files[0].get("captured_path").unwrap().is_null());
    assert!(
        files[0]
            .get("hash")
            .and_then(|value| value.as_str())
            .is_some_and(|hash| hash.starts_with("sha256:"))
    );

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn dir_hook_moves_directory_contents() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-dir");
    let input_dir = temp_dir.join("moveme");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("output.txt"), b"move me").unwrap();

    let config = json!({
        "path": "moveme",
        "mode": "move",
        "hash": "none"
    });
    let hook = <DirHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = test_run(&temp_dir, &run_dir);
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir.clone());

    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let output = captured
        .serialize_json()
        .expect("serialization should succeed");

    let files = output
        .get("files")
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(files.len(), 1, "Should report the moved file");
    assert!(artifact_dir.join("moveme/output.txt").exists());
    assert!(!input_dir.exists(), "Original directory should be moved");

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn dir_hook_errors_when_path_is_file() {
    let temp_dir = std::env::temp_dir().join(format!("capsula_test_{}", Ulid::new()));
    let run_dir = temp_dir.join("run");
    let artifact_dir = run_dir.join("pre-0-capture-dir");
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::write(temp_dir.join("not-a-dir.txt"), b"not a directory").unwrap();

    let config = json!({
        "path": "not-a-dir.txt",
        "mode": "copy",
        "hash": "none"
    });
    let hook = <DirHook as Hook<PreRun>>::from_config(&config, &temp_dir).expect("from_config ok");
    let run_metadata = test_run(&temp_dir, &run_dir);
    let params = RuntimeParams::<PreRun>::with_artifact_dir(artifact_dir);

    let error = hook
        .run(&run_metadata, &params)
        .expect_err("file paths should fail");
    assert!(error.to_string().contains("capture-dir"));

    fs::remove_dir_all(&temp_dir).ok();
}
