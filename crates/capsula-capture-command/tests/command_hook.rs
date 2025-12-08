#![expect(clippy::unwrap_used)]

use capsula_capture_command::CommandHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::path::PathBuf;
use ulid::Ulid;

#[test]
fn command_hook_executes_successful_command() {
    // Arrange
    let config = json!({
        "command": ["echo", "hello world"],
        "abort_on_failure": false
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: PathBuf::from("."),
        project_root: PathBuf::from("."),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
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
    assert!(!captured.abort_requested(), "Should not request abort");
}

#[test]
fn command_hook_captures_failing_command() {
    // Arrange - use 'false' command which always exits with code 1
    let config = json!({
        "command": ["false"],
        "abort_on_failure": false
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: PathBuf::from("."),
        project_root: PathBuf::from("."),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    assert_ne!(
        json.get("status").and_then(serde_json::Value::as_i64),
        Some(0)
    );
    assert!(
        !captured.abort_requested(),
        "Should not request abort when abort_on_failure is false"
    );
}

#[test]
fn command_hook_aborts_on_failure_when_configured() {
    // Arrange
    let config = json!({
        "command": ["false"],
        "abort_on_failure": true
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: PathBuf::from("."),
        project_root: PathBuf::from("."),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");

    // Assert
    assert!(
        captured.abort_requested(),
        "Should request abort when command fails and abort_on_failure is true"
    );
}

#[test]
fn command_hook_captures_stderr() {
    // Arrange - use a command that writes to stderr
    let config = json!({
        "command": ["sh", "-c", "echo 'error message' >&2"],
        "abort_on_failure": false
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: PathBuf::from("."),
        project_root: PathBuf::from("."),
    };
    let params = RuntimeParams::<PreRun>::default();

    // Act
    let captured = hook.run(&run_metadata, &params).expect("run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");

    // Assert
    let stderr = json.get("stderr").and_then(|v| v.as_str()).unwrap();
    assert!(
        stderr.contains("error message"),
        "stderr should contain error message"
    );
}
