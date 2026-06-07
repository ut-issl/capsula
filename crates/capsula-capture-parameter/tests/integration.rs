//! End-to-end tests for the `capture-parameter` hook against a real
//! filesystem (temp dirs).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Tests use unwrap/expect/panic for clarity"
)]

use std::fs;
use std::path::Path;

use capsula_capture_parameter::ParameterHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, PreRun, RuntimeParams};
use capsula_core::run::{PreparedRun, Run};
use serde_json::json;
use tempfile::TempDir;
use ulid::Ulid;

fn make_run(project_root: &Path) -> PreparedRun {
    Run {
        id: Ulid::new(),
        name: "test-run".into(),
        command: vec![],
        run_dir: project_root.to_path_buf(),
        project_root: project_root.to_path_buf(),
    }
}

fn run_hook(
    project_root: &Path,
    config: &serde_json::Value,
) -> capsula_core::error::CapsulaResult<serde_json::Value> {
    let hook = <ParameterHook as Hook<PreRun>>::from_config(config, project_root)?;
    let run = make_run(project_root);
    let params = RuntimeParams::<PreRun>::default();
    let captured = <ParameterHook as Hook<PreRun>>::run(&hook, &run, &params)?;
    Ok(captured.serialize_json().expect("serialize"))
}

/// Flatten an error chain into a single string by appending every
/// `source()` along the way. Capsula's outer error only reports
/// "Hook 'X' failed", so the actual cause lives in nested sources.
fn full_error_text(err: &(dyn std::error::Error + 'static)) -> String {
    let mut s = err.to_string();
    let mut current = err.source();
    while let Some(e) = current {
        s.push_str(" :: ");
        s.push_str(&e.to_string());
        current = e.source();
    }
    s
}

#[test]
fn parses_single_json_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("params.json"), r#"{"a": 1, "b": "x"}"#).unwrap();

    let out = run_hook(tmp.path(), &json!({ "glob": "*.json" })).unwrap();
    assert_eq!(
        out,
        json!({
            "parameters": { "params": { "a": 1, "b": "x" } }
        })
    );
}

#[test]
fn parses_toml_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("config.toml"),
        "name = \"capsula\"\nport = 8080\n",
    )
    .unwrap();

    let out = run_hook(tmp.path(), &json!({ "glob": "*.toml" })).unwrap();
    assert_eq!(
        out,
        json!({
            "parameters": { "config": { "name": "capsula", "port": 8080 } }
        })
    );
}

#[test]
fn merges_json_and_yaml_with_same_stem() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("orbit.json"), r#"{"a": 1}"#).unwrap();
    fs::write(tmp.path().join("orbit.yaml"), "b: 2\n").unwrap();

    let out = run_hook(tmp.path(), &json!({ "glob": "orbit.*" })).unwrap();
    assert_eq!(
        out,
        json!({
            "parameters": { "orbit": { "a": 1, "b": 2 } }
        })
    );
}

#[test]
fn nested_directories_with_strip_prefix() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("config/sat1")).unwrap();
    fs::create_dir_all(tmp.path().join("config/sat2")).unwrap();
    fs::write(tmp.path().join("config/sat1/orbit.json"), r#"{"a": 1}"#).unwrap();
    fs::write(tmp.path().join("config/sat2/orbit.json"), r#"{"a": 3}"#).unwrap();

    let out = run_hook(
        tmp.path(),
        &json!({
            "glob": "config/**/*.json",
            "strip_prefix": "config"
        }),
    )
    .unwrap();
    assert_eq!(
        out,
        json!({
            "parameters": {
                "sat1": { "orbit": { "a": 1 } },
                "sat2": { "orbit": { "a": 3 } }
            }
        })
    );
}

#[test]
fn leaf_and_intermediate_node_merge() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("sat1")).unwrap();
    fs::write(tmp.path().join("sat1.json"), r#"{"x": 1}"#).unwrap();
    fs::write(tmp.path().join("sat1/orbit.json"), r#"{"a": 1}"#).unwrap();

    let out = run_hook(tmp.path(), &json!({ "glob": "**/*.json" })).unwrap();
    assert_eq!(
        out,
        json!({
            "parameters": {
                "sat1": {
                    "x": 1,
                    "orbit": { "a": 1 }
                }
            }
        })
    );
}

#[test]
fn conflict_when_two_files_disagree_on_value() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("p.json"), r#"{"x": 1}"#).unwrap();
    fs::write(tmp.path().join("p.yaml"), "x: 2\n").unwrap();

    let result = run_hook(tmp.path(), &json!({ "glob": "p.*" }));
    let err = result.unwrap_err();
    let msg = full_error_text(&err);
    assert!(
        msg.to_lowercase().contains("conflict"),
        "expected conflict error, got: {msg}"
    );
}

#[test]
fn unsupported_extension_errors() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("data.csv"), "a,b\n1,2\n").unwrap();

    let result = run_hook(tmp.path(), &json!({ "glob": "*.csv" }));
    assert!(result.is_err());
}

#[test]
fn strip_prefix_mismatch_errors() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("etc")).unwrap();
    fs::write(tmp.path().join("etc/foo.json"), r#"{"a": 1}"#).unwrap();

    let result = run_hook(
        tmp.path(),
        &json!({
            "glob": "etc/*.json",
            "strip_prefix": "config"
        }),
    );
    let err = result.unwrap_err();
    let msg = full_error_text(&err);
    assert!(
        msg.contains("strip_prefix"),
        "expected strip_prefix error, got: {msg}"
    );
}
