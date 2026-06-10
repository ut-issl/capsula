//! `capture-json` hook: parse a single JSON file and embed its parsed
//! content in the run output under the `parameters` field.
//!
//! By design this hook captures exactly one file per instance. Compose
//! multiple `capture-json` entries in `capsula.toml` to capture multiple
//! files.

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
    project_root: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct JsonCaptured {
    /// The path as written in the configuration (verbatim, including any
    /// directory components). Useful for distinguishing multiple
    /// `capture-json` outputs that share the same basename.
    file: String,
    /// The parsed content of the file as JSON.
    parameters: serde_json::Value,
}

impl<P> Hook<P> for JsonHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-json";

    type Config = JsonHookConfig;
    type Output = JsonCaptured;

    fn from_config(config: &serde_json::Value, project_root: &Path) -> CapsulaResult<Self> {
        let config: JsonHookConfig =
            serde_json::from_value(config.clone()).map_err(JsonHookError::from)?;
        Ok(Self {
            config,
            project_root: project_root.to_path_buf(),
        })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        let full_path = self.project_root.join(&self.config.path);
        debug!("JsonHook: reading {}", full_path.display());

        let content = std::fs::read_to_string(&full_path).map_err(|source| JsonHookError::Io {
            path: full_path.clone(),
            source,
        })?;

        let parameters: serde_json::Value =
            serde_json::from_str(&content).map_err(JsonHookError::from)?;

        Ok(JsonCaptured {
            file: self.config.path.to_string_lossy().into_owned(),
            parameters,
        })
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
    fn parses_valid_json_into_parameters_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.json"), r#"{"a": 1, "b": "x"}"#).unwrap();

        let hook = make_hook(tmp.path(), "p.json");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.file, "p.json");
        assert_eq!(captured.parameters, serde_json::json!({"a": 1, "b": "x"}));
    }

    #[test]
    fn file_field_preserves_configured_path_verbatim() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("config/sat1")).unwrap();
        fs::write(tmp.path().join("config/sat1/orbit.json"), r#"{"a": 1}"#).unwrap();

        let hook = make_hook(tmp.path(), "config/sat1/orbit.json");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.file, "config/sat1/orbit.json");
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

        assert_eq!(captured.parameters["sat1"]["orbit"]["a"], 1.42);
        assert_eq!(captured.parameters["sat1"]["orbit"]["b"], "LEO");
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
    fn serialized_output_has_file_and_parameters_fields() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.json"), r#"{"x": 42}"#).unwrap();

        let hook = make_hook(tmp.path(), "p.json");
        let captured = run_hook(&hook, tmp.path()).unwrap();
        let json = captured.serialize_json().unwrap();

        assert_eq!(json["file"], "p.json");
        assert_eq!(json["parameters"]["x"], 42);
        assert!(
            json.get("__meta").is_none(),
            "__meta is added by orchestration, not the hook"
        );
    }
}
