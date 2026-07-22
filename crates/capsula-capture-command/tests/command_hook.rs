// This integration test crate is only compiled for test targets.
#![cfg(test)]

use capsula_capture_command::CommandHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use ulid::Ulid;

fn prepared_run() -> PreparedRun {
    PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: PathBuf::from("."),
        project_root: PathBuf::from("."),
    }
}

#[test]
fn command_hook_executes_successful_command() {
    // Arrange
    let config = json!({
        "command": ["echo", "hello world"]
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = prepared_run();
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_eq!(
        json.get("status").and_then(serde_json::Value::as_i64),
        Some(0)
    );
    let stdout = json.get("stdout").and_then(|v| v.as_str()).unwrap();
    assert!(
        stdout.contains("hello world"),
        "stdout should contain output"
    );
    assert!(outcome.is_success(), "successful command should pass");
}

#[test]
fn command_hook_fails_on_unexpected_status_by_default() {
    // Arrange - use 'false' command which always exits with code 1
    let config = json!({
        "command": ["false"]
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = prepared_run();
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_ne!(
        json.get("status").and_then(serde_json::Value::as_i64),
        Some(0)
    );
    assert!(
        outcome.is_failure(),
        "non-zero status should fail unless configured as successful"
    );
}

#[test]
fn command_hook_allows_expected_nonzero_status() {
    // Arrange
    let config = json!({
        "command": ["false"],
        "success_codes": [1]
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = prepared_run();
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert!(
        outcome.is_success(),
        "configured non-zero status should be accepted"
    );
}

#[test]
fn command_hook_preserves_abort_on_failure_false_compatibility() {
    // Arrange
    let config = json!({
        "command": ["false"],
        "abort_on_failure": false
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = prepared_run();
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert!(
        outcome.is_success(),
        "legacy abort_on_failure = false should accept any exit status"
    );
}

#[test]
fn command_hook_captures_stderr() {
    // Arrange - use a command that writes to stderr
    let config = json!({
        "command": ["sh", "-c", "echo 'error message' >&2"]
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = prepared_run();
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    let stderr = json.get("stderr").and_then(|v| v.as_str()).unwrap();
    assert!(
        stderr.contains("error message"),
        "stderr should contain error message"
    );
    assert!(outcome.is_success(), "status 0 should pass");
}

#[test]
fn command_hook_rejects_unknown_config_fields() {
    let config = json!({
        "command": ["true"],
        "abort_on_failure_typo": true
    });

    let result = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(result.is_err(), "unknown config fields should be rejected");
}

#[test]
fn command_hook_rejects_empty_success_codes() {
    let config = json!({
        "command": ["true"],
        "success_codes": []
    });

    let result = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(result.is_err(), "success_codes must not be empty");
}

#[test]
fn command_hook_rejects_conflicting_status_policy() {
    let config = json!({
        "command": ["true"],
        "success_codes": [0],
        "abort_on_failure": true
    });

    let result = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(
        result.is_err(),
        "success_codes and abort_on_failure should not be combined"
    );
}

#[test]
fn command_hook_rejects_cwd_that_escapes_project_root() {
    let parent = std::env::temp_dir().join(format!("capsula_command_test_{}", Ulid::new()));
    let project_root = parent.join("project");
    let outside = parent.join("outside");
    fs::create_dir_all(&project_root).unwrap();
    fs::create_dir_all(&outside).unwrap();

    let config = json!({
        "command": ["true"],
        "cwd": "../outside"
    });

    let result = <CommandHook as Hook<PreRun>>::from_config(&config, &project_root);

    assert!(result.is_err(), "cwd must stay within the project root");

    fs::remove_dir_all(parent).ok();
}
