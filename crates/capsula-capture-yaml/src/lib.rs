//! `capture-yaml` hook: parse a single YAML file and embed its parsed
//! content in the run output under the `content` field.
//!
//! By design this hook captures exactly one file per instance. Compose
//! multiple `capture-yaml` entries in `capsula.toml` to capture multiple
//! files. The path written in the config is preserved as part of the
//! standard `__meta.config.path`, so the captured value does not need
//! to duplicate it.
//!
//! Only fairly flat/simple YAML is supported: a single document whose
//! values map directly onto JSON (mappings, sequences, strings, numbers,
//! booleans, null). YAML-specific features without a JSON equivalent —
//! multi-document streams, tagged values, and the like — are rejected as
//! parse errors.

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
            id: Ulid::new(),
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
