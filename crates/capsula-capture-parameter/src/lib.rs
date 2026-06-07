mod error;

use std::path::{Path, PathBuf};

use capsula_capture_file::error::FileHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::debug;

use crate::error::ParameterHookError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParameterHookConfig {
    /// Glob pattern (relative to `project_root`) selecting parameter files.
    /// Only `.json` / `.toml` / `.yaml` / `.yml` are parsed.
    glob: String,

    /// Optional literal path prefix to strip from each matched file's
    /// relative path before constructing the nested key path. If specified,
    /// every matched file must start with this prefix or the hook fails with
    /// `StripPrefixMismatch`.
    #[serde(default)]
    strip_prefix: Option<String>,
}

#[derive(Debug)]
pub struct ParameterHook {
    config: ParameterHookConfig,
}

#[derive(Debug, Serialize)]
pub struct ParameterCaptured {
    /// Nested parameter map keyed by directory components and file stems.
    /// Multiple files contributing to the same key path are deep-merged;
    /// conflicting leaf values fail with `ParameterConflict`.
    parameters: Map<String, Value>,
}

impl<P> Hook<P> for ParameterHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-parameter";

    type Config = ParameterHookConfig;
    type Output = ParameterCaptured;

    fn from_config(config: &serde_json::Value, _project_root: &Path) -> CapsulaResult<Self> {
        let config: ParameterHookConfig =
            serde_json::from_value(config.clone()).map_err(ParameterHookError::from)?;
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
        let pattern = build_glob_pattern(&metadata.project_root, &self.config.glob);
        debug!("ParameterHook: searching pattern: {pattern}");

        let mut files = collect_files(&pattern).map_err(ParameterHookError::from)?;
        files.sort();
        debug!("ParameterHook: matched {} files", files.len());

        let mut parameters: Map<String, Value> = Map::new();
        let strip = self.config.strip_prefix.as_deref();
        let mut path_stack: Vec<String> = Vec::new();

        for file_path in files {
            let rel = file_path
                .strip_prefix(&metadata.project_root)
                .unwrap_or(&file_path);
            let keys = compute_keys(rel, strip)?;
            let parsed = parse_file(&file_path)?;
            merge_at_keys(&mut parameters, &keys, parsed, &mut path_stack)?;
        }

        Ok(ParameterCaptured { parameters })
    }
}

impl Captured for ParameterCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

fn build_glob_pattern(base: &Path, pattern: &str) -> String {
    base.join(pattern).to_string_lossy().replace('\\', "/")
}

fn collect_files(pattern: &str) -> Result<Vec<PathBuf>, FileHookError> {
    let paths = glob::glob(pattern)?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .collect();
    Ok(paths)
}

/// Build the nested key sequence for a matched file: optional `strip_prefix`
/// is removed, then the remaining directory components plus the file stem
/// become the keys.
fn compute_keys(
    rel_path: &Path,
    strip_prefix: Option<&str>,
) -> Result<Vec<String>, ParameterHookError> {
    let trimmed = if let Some(prefix) = strip_prefix {
        rel_path.strip_prefix(Path::new(prefix)).map_err(|_| {
            ParameterHookError::StripPrefixMismatch {
                prefix: prefix.to_string(),
                path: rel_path.to_string_lossy().into_owned(),
            }
        })?
    } else {
        rel_path
    };

    let mut keys: Vec<String> = trimmed
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    if let Some(stem) = trimmed.file_stem() {
        keys.push(stem.to_string_lossy().into_owned());
    }

    Ok(keys)
}

/// Merge `leaf` into `target` at the nested key path `keys`. Existing objects
/// are deep-merged; conflicting non-object values produce `ParameterConflict`.
fn merge_at_keys(
    target: &mut Map<String, Value>,
    keys: &[String],
    leaf: Value,
    path_stack: &mut Vec<String>,
) -> Result<(), ParameterHookError> {
    let Some((first, rest)) = keys.split_first() else {
        return Ok(());
    };

    path_stack.push(first.clone());

    if rest.is_empty() {
        match target.get_mut(first) {
            Some(existing) => merge_value(existing, leaf, path_stack)?,
            None => {
                target.insert(first.clone(), leaf);
            }
        }
    } else {
        let entry = target
            .entry(first.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(inner) = entry else {
            return Err(ParameterHookError::ParameterConflict(path_stack.join(".")));
        };
        merge_at_keys(inner, rest, leaf, path_stack)?;
    }

    path_stack.pop();
    Ok(())
}

/// Recursively merge `b` into `a`. Objects are merged key-by-key; scalars,
/// arrays, and nulls must be equal or the merge fails with `ParameterConflict`.
fn merge_value(
    a: &mut Value,
    b: Value,
    path_stack: &mut Vec<String>,
) -> Result<(), ParameterHookError> {
    match b {
        Value::Object(mb) => {
            let Value::Object(ma) = a else {
                return Err(ParameterHookError::ParameterConflict(path_stack.join(".")));
            };
            for (k, v) in mb {
                path_stack.push(k.clone());
                match ma.get_mut(&k) {
                    Some(existing) => merge_value(existing, v, path_stack)?,
                    None => {
                        ma.insert(k, v);
                    }
                }
                path_stack.pop();
            }
            Ok(())
        }
        b_val => {
            if *a == b_val {
                Ok(())
            } else {
                Err(ParameterHookError::ParameterConflict(path_stack.join(".")))
            }
        }
    }
}

fn parse_file(path: &Path) -> Result<serde_json::Value, ParameterHookError> {
    let content = std::fs::read_to_string(path).map_err(|source| FileHookError::ReadError {
        path: path.to_path_buf(),
        source,
    })?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match ext.as_str() {
        "json" => Ok(serde_json::from_str(&content)?),
        "toml" => {
            let v: toml::Value = toml::from_str(&content)?;
            Ok(serde_json::to_value(v)?)
        }
        "yaml" | "yml" => {
            let v: serde_yaml::Value = serde_yaml::from_str(&content)?;
            Ok(serde_json::to_value(v)?)
        }
        _ => Err(ParameterHookError::UnsupportedFileType(ext)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "Tests use unwrap/expect/panic for clarity"
    )]

    use super::*;
    use serde_json::json;

    // ---- compute_keys -----------------------------------------------------

    #[test]
    fn compute_keys_simple_file_no_prefix() {
        let keys = compute_keys(Path::new("foo.json"), None).unwrap();
        assert_eq!(keys, vec!["foo".to_string()]);
    }

    #[test]
    fn compute_keys_nested_no_prefix() {
        let keys = compute_keys(Path::new("sat1/orbit.json"), None).unwrap();
        assert_eq!(keys, vec!["sat1".to_string(), "orbit".to_string()]);
    }

    #[test]
    fn compute_keys_with_strip_prefix_removes_layer() {
        let keys = compute_keys(Path::new("config/sat1/orbit.json"), Some("config")).unwrap();
        assert_eq!(keys, vec!["sat1".to_string(), "orbit".to_string()]);
    }

    #[test]
    fn compute_keys_strip_prefix_mismatch_errors() {
        let err = compute_keys(Path::new("etc/foo.json"), Some("config")).unwrap_err();
        assert!(matches!(
            err,
            ParameterHookError::StripPrefixMismatch { .. }
        ));
    }

    // ---- merge_value ------------------------------------------------------

    #[test]
    fn merge_value_disjoint_objects() {
        let mut a = json!({ "x": 1 });
        merge_value(&mut a, json!({ "y": 2 }), &mut Vec::new()).unwrap();
        assert_eq!(a, json!({ "x": 1, "y": 2 }));
    }

    #[test]
    fn merge_value_equal_scalars_ok() {
        let mut a = json!(42);
        merge_value(&mut a, json!(42), &mut Vec::new()).unwrap();
        assert_eq!(a, json!(42));
    }

    #[test]
    fn merge_value_conflicting_scalars_error() {
        let mut a = json!(1);
        let err = merge_value(&mut a, json!(2), &mut vec!["root".into()]).unwrap_err();
        assert!(matches!(err, ParameterHookError::ParameterConflict(ref p) if p == "root"));
    }

    #[test]
    fn merge_value_type_mismatch_error() {
        let mut a = json!({ "x": 1 });
        let err = merge_value(&mut a, json!(42), &mut vec!["k".into()]).unwrap_err();
        assert!(matches!(err, ParameterHookError::ParameterConflict(_)));
    }

    #[test]
    fn merge_value_equal_arrays_ok() {
        let mut a = json!([1, 2, 3]);
        merge_value(&mut a, json!([1, 2, 3]), &mut Vec::new()).unwrap();
        assert_eq!(a, json!([1, 2, 3]));
    }

    #[test]
    fn merge_value_different_arrays_error() {
        let mut a = json!([1, 2]);
        let err = merge_value(&mut a, json!([1, 3]), &mut vec!["arr".into()]).unwrap_err();
        assert!(matches!(err, ParameterHookError::ParameterConflict(ref p) if p == "arr"));
    }

    #[test]
    fn merge_value_deep_object_merge() {
        let mut a = json!({ "inner": { "x": 1 } });
        merge_value(&mut a, json!({ "inner": { "y": 2 } }), &mut Vec::new()).unwrap();
        assert_eq!(a, json!({ "inner": { "x": 1, "y": 2 } }));
    }

    // ---- merge_at_keys ----------------------------------------------------

    #[test]
    fn merge_at_keys_inserts_at_leaf() {
        let mut target = Map::new();
        merge_at_keys(&mut target, &["foo".into()], json!(1), &mut Vec::new()).unwrap();
        assert_eq!(Value::Object(target), json!({ "foo": 1 }));
    }

    #[test]
    fn merge_at_keys_creates_nested_objects() {
        let mut target: Map<String, Value> = Map::new();
        merge_at_keys(
            &mut target,
            &["sat1".into(), "orbit".into()],
            json!({ "a": 1 }),
            &mut Vec::new(),
        )
        .unwrap();
        merge_at_keys(
            &mut target,
            &["sat1".into(), "attitude".into()],
            json!({ "q": [0, 0, 0, 1] }),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(
            Value::Object(target),
            json!({
                "sat1": {
                    "attitude": { "q": [0, 0, 0, 1] },
                    "orbit":    { "a": 1 }
                }
            })
        );
    }

    #[test]
    fn merge_at_keys_unifies_leaf_and_intermediate() {
        let mut target: Map<String, Value> = Map::new();
        merge_at_keys(
            &mut target,
            &["sat1".into()],
            json!({ "x": 1 }),
            &mut Vec::new(),
        )
        .unwrap();
        merge_at_keys(
            &mut target,
            &["sat1".into(), "orbit".into()],
            json!({ "a": 1 }),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(
            Value::Object(target),
            json!({
                "sat1": {
                    "x": 1,
                    "orbit": { "a": 1 }
                }
            })
        );
    }

    #[test]
    fn merge_at_keys_conflict_reports_full_path() {
        let mut target: Map<String, Value> = Map::new();
        merge_at_keys(
            &mut target,
            &["sat1".into(), "orbit".into()],
            json!({ "a": 1 }),
            &mut Vec::new(),
        )
        .unwrap();
        let err = merge_at_keys(
            &mut target,
            &["sat1".into(), "orbit".into()],
            json!({ "a": 2 }),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(
            matches!(err, ParameterHookError::ParameterConflict(ref p) if p == "sat1.orbit.a"),
            "expected conflict at 'sat1.orbit.a', got: {err:?}"
        );
    }
}
