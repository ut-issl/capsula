// This integration test crate is only compiled for test targets.
#![cfg(test)]

use capsula_capture_env::EnvVarHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::env;
use std::path::PathBuf;
use ulid::Ulid;

#[test]
#[expect(
    unsafe_code,
    reason = "Modifying the environment is unsafe, but acceptable in this test context"
)]
fn env_hook_captures_existing_variable() {
    // Arrange
    let test_var = "CAPSULA_TEST_ENV_VAR";
    let test_value = "test_value_12345";

    // SAFETY: Modifying environment variables is inherently unsafe in a multi-threaded context.
    // However, for the purpose of this test, we ensure that no other threads are modifying
    // the same environment variable concurrently.
    unsafe {
        env::set_var(test_var, test_value);
    }

    let config = json!({
        "name": test_var
    });
    let hook = <EnvVarHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
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
    assert_eq!(json.get("value").and_then(|v| v.as_str()), Some(test_value));

    // Cleanup
    // SAFETY: See above comment regarding environment variable safety.
    unsafe {
        env::remove_var(test_var);
    }
}

#[test]
#[expect(
    unsafe_code,
    reason = "Modifying the environment is unsafe, but acceptable in this test context"
)]
fn env_hook_captures_missing_variable() {
    // Arrange
    let test_var = "CAPSULA_NONEXISTENT_VAR_XYZ";
    // SAFETY: Modifying environment variables is inherently unsafe in a multi-threaded context.
    // However, for the purpose of this test, we ensure that no other threads are modifying
    // the same environment variable concurrently.
    unsafe {
        env::remove_var(test_var); // Ensure it doesn't exist
    }

    let config = json!({
        "name": test_var
    });
    let hook = <EnvVarHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
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
    assert!(json.get("value").unwrap().is_null());
}

#[test]
fn env_hook_rejects_unknown_config_fields() {
    let config = json!({
        "name": "PATH",
        "unexpected": true
    });

    let result = <EnvVarHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(result.is_err(), "unknown config fields should be rejected");
}
