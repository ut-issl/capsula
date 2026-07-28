// This integration test crate is only compiled for test targets.
#![cfg(test)]

use capsula_capture_cwd::CwdHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::path::PathBuf;
use ulid::Ulid;

#[test]
fn cwd_hook_captures_current_dir_and_json() {
    // Arrange
    let expected = std::env::current_dir().expect("current_dir");
    let hook = CwdHook::default();
    let run_metadata = PreparedRun {
        id: Ulid::generate(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: expected.clone(),
        project_root: expected.clone(),
    };
    let params = RuntimeParams::<PreRun>::default();
    // Act
    let captured = hook.run(&run_metadata, &params).expect("CwdHook::run ok");
    let json = captured
        .serialize_json()
        .expect("serialization should succeed");
    let json_cwd = json
        .get("cwd")
        .and_then(|v| v.as_str())
        .expect("json has 'cwd' string");

    // Assert (captured struct)
    assert_eq!(
        captured.cwd_abs(),
        expected.as_path(),
        "cwd_abs should match current_dir"
    );

    // Assert (JSON view)
    assert_eq!(
        json_cwd,
        expected.to_string_lossy(),
        "JSON 'cwd' should match current_dir string"
    );
}

#[test]
fn cwd_hook_rejects_unknown_config_fields() {
    let config = json!({
        "unexpected": true
    });

    let result = <CwdHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(result.is_err(), "unknown config fields should be rejected");
}
