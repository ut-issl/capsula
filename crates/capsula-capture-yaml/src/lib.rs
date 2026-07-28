//! `capture-yaml` hook: parse a single YAML file and embed its parsed
//! content in the run output under the `content` field.
//!
//! By design this hook captures exactly one file per instance. Compose
//! multiple `capture-yaml` entries in `capsula.toml` to capture multiple
//! files. The path written in the config is preserved as part of the
//! standard `__meta.config.path`, so the captured value does not need
//! to duplicate it.
//!
//! The capture contract is intentionally minimal: the output is exactly
//! what `yaml_serde`'s untyped deserialization into `serde_json::Value`
//! produces, no more and no less. The behaviors at the YAML/JSON
//! boundary are pinned by the tests in this crate and documented in
//! `docs/hooks/capture-yaml.md`:
//!
//! - Multi-document streams are rejected as parse errors.
//! - Anchors and aliases are expanded; merge keys (`<<`) are NOT
//!   applied and are captured as literal `"<<"` entries.
//! - Non-string scalar mapping keys are stringified; keys that collide
//!   after stringification (e.g. `1` and `"1"`) follow last-write-wins,
//!   silently dropping one value.
//! - `!!binary` values are captured as their raw base64 strings; custom
//!   tags (e.g. `!Custom`) and non-scalar mapping keys are rejected.
//! - Non-finite floats (`.nan`, `.inf`) become JSON `null`.
//!
//! Keep captured files to plain, JSON-representable YAML to stay clear
//! of these edge cases.

mod error;

use std::path::{Path, PathBuf};

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::YamlHookError;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct YamlHookConfig {
    /// Path to the YAML file to parse, relative to `project_root`.
    /// Absolute paths are also accepted.
    path: PathBuf,
}

#[derive(Debug)]
pub struct YamlHook {
    config: YamlHookConfig,
}

#[derive(Debug, Serialize)]
pub struct YamlCaptured {
    /// The parsed content of the file, converted to JSON.
    content: serde_json::Value,
}

impl<P> Hook<P> for YamlHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-yaml";

    type Config = YamlHookConfig;
    type Output = YamlCaptured;

    fn from_config(config: &serde_json::Value, _project_root: &Path) -> CapsulaResult<Self> {
        let config: YamlHookConfig =
            serde_json::from_value(config.clone()).map_err(YamlHookError::from)?;
        Ok(Self { config })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        let full_path = metadata.project_root.join(&self.config.path);
        debug!("YamlHook: reading {}", full_path.display());

        let raw = std::fs::read_to_string(&full_path).map_err(|source| YamlHookError::Io {
            path: full_path.clone(),
            source,
        })?;

        let content: serde_json::Value = yaml_serde::from_str(&raw).map_err(YamlHookError::from)?;

        Ok(YamlCaptured { content })
    }
}

impl Captured for YamlCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "Tests use unwrap/expect for brevity"
    )]

    use super::*;
    use capsula_core::hook::PreRun;
    use capsula_core::run::Run;
    use std::fs;
    use tempfile::TempDir;
    use ulid::Ulid;

    fn make_hook(project_root: &Path, path: &str) -> YamlHook {
        let cfg = serde_json::json!({ "path": path });
        <YamlHook as Hook<PreRun>>::from_config(&cfg, project_root).unwrap()
    }

    fn make_run(project_root: &Path) -> PreparedRun {
        Run {
            id: Ulid::generate(),
            name: "test-run".into(),
            command: vec![],
            run_dir: project_root.to_path_buf(),
            project_root: project_root.to_path_buf(),
        }
    }

    fn run_hook(hook: &YamlHook, project_root: &Path) -> CapsulaResult<YamlCaptured> {
        let run = make_run(project_root);
        <YamlHook as Hook<PreRun>>::run(hook, &run, &RuntimeParams::default())
    }

    #[test]
    fn parses_valid_yaml_into_content_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.yaml"), "name: capsula\nport: 8080\n").unwrap();

        let hook = make_hook(tmp.path(), "p.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(
            captured.content,
            serde_json::json!({"name": "capsula", "port": 8080})
        );
    }

    #[test]
    fn resolves_path_relative_to_metadata_project_root() {
        // The hook reads project_root from PreparedRun, not from its own
        // state. Capture this contract: a hook built against tmp_a but
        // run against tmp_b should resolve relative to tmp_b.
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        fs::write(tmp_b.path().join("p.yaml"), "x: 42\n").unwrap();

        let hook = make_hook(tmp_a.path(), "p.yaml");
        let captured = run_hook(&hook, tmp_b.path()).unwrap();

        assert_eq!(captured.content, serde_json::json!({"x": 42}));
    }

    #[test]
    fn supports_nested_mappings_and_sequences() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("nested.yaml"),
            r"
sat1:
  orbit:
    a: 1.42
    b: LEO
  payloads:
    - camera
    - antenna
",
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "nested.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.content["sat1"]["orbit"]["a"], 1.42);
        assert_eq!(captured.content["sat1"]["orbit"]["b"], "LEO");
        assert_eq!(
            captured.content["sat1"]["payloads"],
            serde_json::json!(["camera", "antenna"])
        );
    }

    #[test]
    fn missing_file_returns_io_error() {
        let tmp = TempDir::new().unwrap();
        let hook = make_hook(tmp.path(), "does-not-exist.yaml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn invalid_yaml_returns_parse_error() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("bad.yaml"), "key: [unclosed\n").unwrap();

        let hook = make_hook(tmp.path(), "bad.yaml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn multi_document_yaml_is_rejected() {
        // Multi-document streams have no JSON equivalent and are out of
        // scope for this hook; they should surface as a parse error.
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("multi.yaml"), "---\na: 1\n---\nb: 2\n").unwrap();

        let hook = make_hook(tmp.path(), "multi.yaml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    // The tests below pin the yaml_serde edge-case behaviors that define
    // this hook's capture contract (see the crate docs and
    // docs/hooks/capture-yaml.md). If a yaml_serde upgrade changes any of
    // these, the docs must be updated in the same PR.

    #[test]
    fn anchors_and_aliases_are_expanded() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("alias.yaml"),
            "base: &b\n  x: 1\ncopy: *b\n",
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "alias.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.content["copy"]["x"], 1);
    }

    #[test]
    fn merge_keys_are_captured_literally_not_applied() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("merge.yaml"),
            "defaults: &defaults\n  retries: 3\njob:\n  <<: *defaults\n  timeout: 5\n",
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "merge.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        // Merge semantics are NOT applied: `<<` stays as an ordinary key
        // and `retries` is not merged into `job`.
        assert_eq!(captured.content["job"]["<<"]["retries"], 3);
        assert!(captured.content["job"].get("retries").is_none());
        assert_eq!(captured.content["job"]["timeout"], 5);
    }

    #[test]
    fn non_string_scalar_keys_are_stringified_last_write_wins() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("keys.yaml"), "1: numeric\n\"1\": string\n").unwrap();

        let hook = make_hook(tmp.path(), "keys.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        // The integer key `1` is stringified and then overwritten by the
        // explicit string key `"1"`: last write wins, one value is lost.
        assert_eq!(captured.content, serde_json::json!({"1": "string"}));
    }

    #[test]
    fn binary_tags_are_captured_as_base64_strings() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("bin.yaml"), "payload: !!binary SGVsbG8=\n").unwrap();

        let hook = make_hook(tmp.path(), "bin.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        // The base64 payload is captured as-is, without the tag or any
        // decoding.
        assert_eq!(captured.content["payload"], "SGVsbG8=");
    }

    #[test]
    fn custom_tags_are_rejected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("tag.yaml"), "value: !Custom 1\n").unwrap();

        let hook = make_hook(tmp.path(), "tag.yaml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn non_scalar_mapping_keys_are_rejected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("seqkey.yaml"), "[1, 2]: value\n").unwrap();

        let hook = make_hook(tmp.path(), "seqkey.yaml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn non_finite_floats_become_null() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("floats.yaml"), "a: .nan\nb: .inf\n").unwrap();

        let hook = make_hook(tmp.path(), "floats.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.content["a"], serde_json::Value::Null);
        assert_eq!(captured.content["b"], serde_json::Value::Null);
    }

    #[test]
    fn serialized_output_has_only_content_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.yaml"), "x: 42\n").unwrap();

        let hook = make_hook(tmp.path(), "p.yaml");
        let captured = run_hook(&hook, tmp.path()).unwrap();
        let json = captured.serialize_json().unwrap();

        assert_eq!(json["content"]["x"], 42);
        assert!(
            json.get("file").is_none(),
            "file is redundant with __meta.config.path"
        );
        assert!(
            json.get("__meta").is_none(),
            "__meta is added by orchestration, not the hook"
        );
    }

    #[test]
    fn rejects_unknown_config_fields() {
        let config = serde_json::json!({
            "path": "p.yaml",
            "unexpected": true,
        });

        let result = <YamlHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

        assert!(result.is_err(), "unknown config fields should be rejected");
    }
}
