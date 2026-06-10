//! `capture-toml` hook: parse a single TOML file and embed its parsed
//! content in the run output under the `parameters` field.
//!
//! By design this hook captures exactly one file per instance. Compose
//! multiple `capture-toml` entries in `capsula.toml` to capture multiple
//! files.

mod error;

use std::path::{Path, PathBuf};

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::TomlHookError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlHookConfig {
    /// Path to the TOML file to parse, relative to `project_root`.
    /// Absolute paths are also accepted.
    path: PathBuf,
}

#[derive(Debug)]
pub struct TomlHook {
    config: TomlHookConfig,
    project_root: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct TomlCaptured {
    /// The path as written in the configuration (verbatim, including any
    /// directory components). Useful for distinguishing multiple
    /// `capture-toml` outputs that share the same basename.
    file: String,
    /// The parsed content of the file, converted to JSON. TOML datetime
    /// values are emitted as RFC 3339 strings.
    parameters: serde_json::Value,
}

impl<P> Hook<P> for TomlHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-toml";

    type Config = TomlHookConfig;
    type Output = TomlCaptured;

    fn from_config(config: &serde_json::Value, project_root: &Path) -> CapsulaResult<Self> {
        let config: TomlHookConfig =
            serde_json::from_value(config.clone()).map_err(TomlHookError::from)?;
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
        debug!("TomlHook: reading {}", full_path.display());

        let content = std::fs::read_to_string(&full_path).map_err(|source| TomlHookError::Io {
            path: full_path.clone(),
            source,
        })?;

        let toml_value: toml::Value = toml::from_str(&content).map_err(TomlHookError::from)?;
        let parameters = toml_value_to_json(toml_value);

        Ok(TomlCaptured {
            file: self.config.path.to_string_lossy().into_owned(),
            parameters,
        })
    }
}

impl Captured for TomlCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// Convert a `toml::Value` to a `serde_json::Value` without going through
/// the toml crate's serde Serialize impl, which wraps datetimes in a
/// `{"$__toml_private_datetime": "..."}` marker object that would leak into
/// query paths. Datetimes are emitted as RFC 3339 strings; floats that are
/// NaN or +/-infinity become JSON null (JSON cannot represent them).
fn toml_value_to_json(value: toml::Value) -> serde_json::Value {
    use serde_json::Value as J;
    use toml::Value as T;
    match value {
        T::String(s) => J::String(s),
        T::Integer(i) => J::Number(i.into()),
        T::Float(f) => serde_json::Number::from_f64(f).map_or(J::Null, J::Number),
        T::Boolean(b) => J::Bool(b),
        T::Datetime(dt) => J::String(dt.to_string()),
        T::Array(arr) => J::Array(arr.into_iter().map(toml_value_to_json).collect()),
        T::Table(table) => J::Object(
            table
                .into_iter()
                .map(|(k, v)| (k, toml_value_to_json(v)))
                .collect(),
        ),
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

    fn make_hook(project_root: &Path, path: &str) -> TomlHook {
        let cfg = serde_json::json!({ "path": path });
        <TomlHook as Hook<PreRun>>::from_config(&cfg, project_root).unwrap()
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

    fn run_hook(hook: &TomlHook, project_root: &Path) -> CapsulaResult<TomlCaptured> {
        let run = make_run(project_root);
        <TomlHook as Hook<PreRun>>::run(hook, &run, &RuntimeParams::default())
    }

    #[test]
    fn parses_valid_toml_into_parameters_field() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("p.toml"),
            "name = \"capsula\"\nport = 8080\n",
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "p.toml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.file, "p.toml");
        assert_eq!(
            captured.parameters,
            serde_json::json!({"name": "capsula", "port": 8080})
        );
    }

    #[test]
    fn file_field_preserves_configured_path_verbatim() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("config/sat1")).unwrap();
        fs::write(
            tmp.path().join("config/sat1/orbit.toml"),
            "a = 1.42\nb = \"LEO\"\n",
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "config/sat1/orbit.toml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.file, "config/sat1/orbit.toml");
    }

    #[test]
    fn supports_nested_toml_tables() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("nested.toml"),
            r#"
[sat1.orbit]
a = 1.42
b = "LEO"
"#,
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "nested.toml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert_eq!(captured.parameters["sat1"]["orbit"]["a"], 1.42);
        assert_eq!(captured.parameters["sat1"]["orbit"]["b"], "LEO");
    }

    #[test]
    fn toml_datetime_is_emitted_as_string() {
        // TOML has dedicated datetime types but JSON does not; toml's serde
        // serializer emits RFC 3339 strings.
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("dt.toml"),
            "created_at = 2026-01-08T10:20:00Z\n",
        )
        .unwrap();

        let hook = make_hook(tmp.path(), "dt.toml");
        let captured = run_hook(&hook, tmp.path()).unwrap();

        assert!(
            captured.parameters["created_at"].is_string(),
            "TOML datetime should be coerced to a JSON string, got: {:?}",
            captured.parameters["created_at"]
        );
    }

    #[test]
    fn missing_file_returns_io_error() {
        let tmp = TempDir::new().unwrap();
        let hook = make_hook(tmp.path(), "does-not-exist.toml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn invalid_toml_returns_parse_error() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("bad.toml"), "not valid toml = =").unwrap();

        let hook = make_hook(tmp.path(), "bad.toml");
        assert!(run_hook(&hook, tmp.path()).is_err());
    }

    #[test]
    fn serialized_output_has_file_and_parameters_fields() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("p.toml"), "x = 42\n").unwrap();

        let hook = make_hook(tmp.path(), "p.toml");
        let captured = run_hook(&hook, tmp.path()).unwrap();
        let json = captured.serialize_json().unwrap();

        assert_eq!(json["file"], "p.toml");
        assert_eq!(json["parameters"]["x"], 42);
        assert!(
            json.get("__meta").is_none(),
            "__meta is added by orchestration, not the hook"
        );
    }
}
