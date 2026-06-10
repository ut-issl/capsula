//! `capture-json` hook: parse a single JSON file and embed its parsed
//! content in the run output under the `content` field.
//!
//! By design this hook captures exactly one file per instance. Compose
//! multiple `capture-json` entries in `capsula.toml` to capture multiple
//! files. The path written in the config is preserved as part of the
//! standard `__meta.config.path`, so the captured value does not need
//! to duplicate it.

mod error;

use std::path::{Path, PathBuf};

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::JsonHookError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonHookConfig {
    /// Path to the JSON file to parse, relative to `project_root`.
    /// Absolute paths are also accepted.
    path: PathBuf,
}

#[derive(Debug)]
pub struct JsonHook {
    config: JsonHookConfig,
}

#[derive(Debug, Serialize)]
pub struct JsonCaptured {
    /// The parsed content of the file as JSON.
    content: serde_json::Value,
}

impl<P> Hook<P> for JsonHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-json";

    type Config = JsonHookConfig;
    type Output = JsonCaptured;

    fn from_config(config: &serde_json::Value, _project_root: &Path) -> CapsulaResult<Self> {
        let config: JsonHookConfig =
            serde_json::from_value(config.clone()).map_err(JsonHookError::from)?;
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
        debug!("JsonHook: reading {}", full_path.display());

        let raw = std::fs::read_to_string(&full_path).map_err(|source| JsonHookError::Io {
            path: full_path.clone(),
            source,
        })?;

        let content: serde_json::Value = serde_json::from_str(&raw).map_err(JsonHookError::from)?;

        Ok(JsonCaptured { content })
    }
}

impl Captured for JsonCaptured {
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

    fn make_hook(project_root: &Path, path: &str) -> JsonHook {
        let cfg = serde_json::json!({ "path": path });
        <JsonHook as Hook<PreRun>>::from_config(&cfg, project_root).unwrap()
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

    fn run_hook(hook: &JsonHook, project_root: &Path) -> CapsulaResult<JsonCaptured> {
        let run = make_run(project_root);
        <JsonHook as Hook<PreRun>>::run(hook, &run, &RuntimeParams::default())
    }

    #[test]
    fn parses_valid_json_into_content_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.json"), r#"{"a": 1, "b": "x"}"#).unwrap();

        let hook = make_hook(tmp.path(), "p.json");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.content, serde_json::json!({"a": 1, "b": "x"}));
    }

    #[test]
    fn resolves_path_relative_to_metadata_project_root() {
        // The hook reads project_root from PreparedRun, not from its own
        // state. Capture this contract: a hook built against tmp_a but
        // run against tmp_b should resolve relative to tmp_b.
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        fs::write(tmp_b.path().join("p.json"), r#"{"x": 1}"#).unwrap();

        let hook = make_hook(tmp_a.path(), "p.json");
        let captured = run_hook(&hook, tmp_b.path()).unwrap();

        assert_eq!(captured.content, serde_json::json!({"x": 1}));
    }

    #[test]
    fn supports_nested_json_objects() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("nested.json"),
            r#"{"sat1": {"orbit": {"a": 1.42, "b": "LEO"}}}"#,
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "nested.json");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.content["sat1"]["orbit"]["a"], 1.42);
        assert_eq!(captured.content["sat1"]["orbit"]["b"], "LEO");
    }

    #[test]
    fn missing_file_returns_io_error() {
        let tmp = TempDir::new().unwrap();
        let hook = make_hook(tmp.path(), "does-not-exist.json");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn invalid_json_returns_parse_error() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("bad.json"), "not json{").unwrap();

        let hook = make_hook(tmp.path(), "bad.json");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn serialized_output_has_only_content_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.json"), r#"{"x": 42}"#).unwrap();

        let hook = make_hook(tmp.path(), "p.json");
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
}
