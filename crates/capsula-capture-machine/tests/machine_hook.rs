// This integration test crate is only compiled for test targets.
#![cfg(test)]

use capsula_capture_machine::MachineHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::path::PathBuf;
use ulid::Ulid;

#[test]
fn machine_hook_captures_system_info() {
    // Arrange
    let config = json!({});
    let hook = <MachineHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
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
    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    // Assert - verify expected fields exist
    assert!(json.get("os").is_some(), "Should capture OS");
    assert!(
        json.get("os_version").is_some(),
        "Should capture OS version"
    );
    assert!(
        json.get("kernel_version").is_some(),
        "Should capture kernel version"
    );
    assert!(
        json.get("architecture").is_some(),
        "Should capture architecture"
    );
    assert!(json.get("cpus").is_some(), "Should capture CPU info");
    assert!(
        json.get("total_memory").is_some(),
        "Should capture total memory"
    );
    assert!(json.get("hostname").is_some(), "Should capture hostname");

    // Verify CPU array is not empty
    let cpus = json.get("cpus").unwrap().as_array().unwrap();
    assert!(!cpus.is_empty(), "Should have at least one CPU");

    // Verify total_memory is a positive number
    let total_memory = json.get("total_memory").unwrap().as_u64().unwrap();
    assert!(total_memory > 0, "Total memory should be positive");
}

#[test]
fn machine_hook_default_config() {
    // Test that default config works
    let hook = MachineHook::default();
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: PathBuf::from("."),
        project_root: PathBuf::from("."),
    };
    let params = RuntimeParams::<PreRun>::default();

    let outcome = hook.run(&run_metadata, &params).expect("run ok");
    let json = outcome
        .output()
        .serialize_json()
        .expect("serialization should succeed");

    assert!(json.is_object(), "Should return valid JSON object");
}

#[test]
fn machine_hook_rejects_unknown_config_fields() {
    let config = json!({
        "unexpected": true
    });

    let result = <MachineHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    assert!(result.is_err(), "unknown config fields should be rejected");
}
