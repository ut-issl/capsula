// This integration test crate is only compiled for test targets.
#![cfg(test)]

use capsula_capture_command::CommandHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::path::PathBuf;
use ulid::Ulid;

fn prepared_run() -> PreparedRun {
    PreparedRun {
        id: Ulid::generate(),
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
fn command_hook_accepts_any_status_with_any() {
    // Arrange
    let config = json!({
        "command": ["false"],
        "success_codes": "any"
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
        "success_codes = \"any\" should accept any exit status"
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
fn command_hook_rejects_removed_abort_on_failure_with_migration_hint() {
    for abort_on_failure in [false, true] {
        let config = json!({
            "command": ["true"],
            "abort_on_failure": abort_on_failure
        });

        let err = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
            .expect_err("abort_on_failure should be rejected");

        let message = std::error::Error::source(&err).map(ToString::to_string);
        assert!(
            message
                .as_deref()
                .is_some_and(|m| m.contains(r#"success_codes = "any""#)),
            "the error should name the replacement: {message:?}"
        );
    }
}

#[test]
fn command_hook_config_records_default_success_codes() {
    let config = json!({ "command": ["true"] });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let serialized =
        serde_json::to_value(<CommandHook as Hook<PreRun>>::config(&hook)).expect("serialize ok");

    assert_eq!(serialized.get("success_codes"), Some(&json!([0])));
}

#[test]
fn command_hook_config_keeps_unset_cwd_as_null() {
    // `__meta.config` is searched by the server's `config_filter`, so its shape
    // must stay the same as in earlier runs.
    let config = json!({
        "command": ["true"]
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config ok");

    let serialized =
        serde_json::to_value(<CommandHook as Hook<PreRun>>::config(&hook)).expect("serialize ok");

    assert_eq!(serialized.get("cwd"), Some(&serde_json::Value::Null));
}

#[test]
fn command_hook_rejects_empty_command_when_built() {
    let config = json!({ "command": [] });

    let result = <CommandHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(result.is_err(), "an empty command should be rejected");
}

#[test]
fn command_hook_resolves_cwd_when_run() {
    // A post-run hook may refer to a directory that the command creates, so a
    // missing `cwd` is an error of the run, not of the configuration.
    let project_root =
        std::env::temp_dir().join(format!("capsula_command_test_{}", Ulid::generate()));
    std::fs::create_dir_all(&project_root).expect("create project root");
    let config = json!({
        "command": ["pwd"],
        "cwd": "created-later"
    });
    let hook = <CommandHook as Hook<PreRun>>::from_config(&config, &project_root)
        .expect("a missing cwd should not fail the configuration");
    let params = RuntimeParams::<PreRun>::default();

    let err = <CommandHook as Hook<PreRun>>::run(&hook, &prepared_run(), &params)
        .expect_err("running in a missing cwd should fail");
    let message = std::error::Error::source(&err).map(ToString::to_string);
    assert!(
        message
            .as_deref()
            .is_some_and(|m| m.contains("created-later")),
        "the error should name the directory: {message:?}"
    );

    std::fs::create_dir(project_root.join("created-later")).expect("create cwd");
    let outcome = <CommandHook as Hook<PreRun>>::run(&hook, &prepared_run(), &params)
        .expect("running in an existing cwd should succeed");
    assert!(outcome.is_success());

    std::fs::remove_dir_all(&project_root).ok();
}
