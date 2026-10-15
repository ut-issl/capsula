//! Per-hook results and per-phase reports.
//!
//! A [`HookRecord`] is the single source of truth for one hook's result. The
//! `__meta` object written to `pre-run.json` / `post-run.json`, the decision
//! whether the phase failed, and the messages shown to users are all derived
//! from it, so they cannot disagree with each other.

use anyhow::{Context, Result, bail};
use capsula_core::captured::Captured;
use capsula_core::hook::{HookOutcome, PhaseMarker};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::marker::PhantomData;

/// Key under which hook metadata is stored next to the captured output.
const META_KEY: &str = "__meta";

/// How a single hook ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookStatus {
    /// The hook ran and its configured success condition passed.
    Succeeded,
    /// The hook ran and captured output, but its configured success condition failed.
    Failed { reason: String },
    /// The hook could not complete, or its result could not be recorded.
    Errored { error: String },
}

impl HookStatus {
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }

    /// The failure reason or error message, or `None` when the hook succeeded.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Succeeded => None,
            Self::Failed { reason } => Some(reason),
            Self::Errored { error } => Some(error),
        }
    }
}

/// The recorded result of one hook in a phase.
#[derive(Debug, Clone, PartialEq)]
pub struct HookRecord {
    /// 0-based position of the hook in the phase's configuration.
    pub index: usize,
    pub hook_id: String,
    /// Serialized hook configuration; `null` when it could not be serialized.
    pub config: Value,
    /// Captured output; `None` when the hook errored before producing one.
    pub output: Option<Map<String, Value>>,
    pub status: HookStatus,
}

/// Shape of the `__meta` object as stored on disk, including records written
/// by older Capsula versions (which omit `id` on errors and have no
/// `failure_reason`).
#[derive(Deserialize)]
struct StoredMeta {
    id: Option<String>,
    #[serde(default)]
    config: Value,
    success: bool,
    error: Option<String>,
    failure_reason: Option<String>,
}

impl HookRecord {
    /// A record for a hook that errored without producing output.
    #[must_use]
    pub fn errored(
        index: usize,
        hook_id: impl Into<String>,
        config: Value,
        error: impl Into<String>,
    ) -> Self {
        Self {
            index,
            hook_id: hook_id.into(),
            config,
            output: None,
            status: HookStatus::Errored {
                error: error.into(),
            },
        }
    }

    /// Build a record from the result of running a hook.
    ///
    /// `result` is `Err` with a formatted message when the hook returned an
    /// operational error. A captured output that cannot be serialized makes
    /// the record [`HookStatus::Errored`], so an unrecordable result never
    /// counts as a success.
    #[must_use]
    pub fn from_run(
        index: usize,
        hook_id: impl Into<String>,
        config: Value,
        result: Result<HookOutcome<Box<dyn Captured>>, String>,
    ) -> Self {
        let hook_id = hook_id.into();
        let (output, status) = match result {
            Ok(HookOutcome::Succeeded(output)) => (output, HookStatus::Succeeded),
            Ok(HookOutcome::Failed { output, reason }) => (output, HookStatus::Failed { reason }),
            Err(error) => return Self::errored(index, hook_id, config, error),
        };
        match output.serialize_json() {
            Ok(json) => Self {
                index,
                hook_id,
                config,
                output: Some(into_object(json)),
                status,
            },
            Err(e) => Self::errored(
                index,
                hook_id,
                config,
                format!("Failed to serialize captured data: {e}"),
            ),
        }
    }

    /// Read a record back from one element of `pre-run.json` / `post-run.json`.
    ///
    /// Records are interpreted as they were written: older runs recorded
    /// policy violations (e.g. a dirty repository) as `success: true`, so they
    /// read back as [`HookStatus::Succeeded`]. Records without an `id` (older
    /// error records) get the hook ID `"unknown"`.
    pub fn from_json(index: usize, value: &Value) -> Result<Self> {
        let Value::Object(map) = value else {
            bail!("hook result #{index} is not a JSON object");
        };
        let mut output = map.clone();
        let meta = output
            .remove(META_KEY)
            .with_context(|| format!("hook result #{index} has no {META_KEY} field"))?;
        let meta: StoredMeta = serde_json::from_value(meta)
            .with_context(|| format!("hook result #{index} has an invalid {META_KEY} field"))?;

        let status = match (meta.success, meta.error, meta.failure_reason) {
            (true, _, _) => HookStatus::Succeeded,
            (false, Some(error), _) => HookStatus::Errored { error },
            (false, None, Some(reason)) => HookStatus::Failed { reason },
            (false, None, None) => HookStatus::Errored {
                error: "unknown error".to_string(),
            },
        };
        let output = match status {
            HookStatus::Errored { .. } => None,
            HookStatus::Succeeded | HookStatus::Failed { .. } => Some(output),
        };

        Ok(Self {
            index,
            hook_id: meta.id.unwrap_or_else(|| "unknown".to_string()),
            config: meta.config,
            output,
            status,
        })
    }

    /// The JSON written to `pre-run.json` / `post-run.json`: the captured
    /// output with a `__meta` object attached.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut meta = Map::new();
        meta.insert("id".to_string(), json!(self.hook_id));
        meta.insert("config".to_string(), self.config.clone());
        meta.insert("success".to_string(), json!(self.status.is_success()));
        match &self.status {
            HookStatus::Succeeded => {}
            HookStatus::Failed { reason } => {
                meta.insert("failure_reason".to_string(), json!(reason));
            }
            HookStatus::Errored { error } => {
                meta.insert("error".to_string(), json!(error));
            }
        }

        let mut json = self.output.clone().unwrap_or_default();
        json.insert(META_KEY.to_string(), Value::Object(meta));
        Value::Object(json)
    }
}

/// Wrap a non-object output so metadata can be attached next to it.
fn into_object(json: Value) -> Map<String, Value> {
    match json {
        Value::Object(map) => map,
        other => Map::from_iter([("value".to_string(), other)]),
    }
}

/// The records of every hook in one phase, in configuration order.
#[derive(Debug, Clone)]
pub struct PhaseReport<P: PhaseMarker> {
    records: Vec<HookRecord>,
    phase: PhantomData<P>,
}

impl<P: PhaseMarker> PhaseReport<P> {
    #[must_use]
    pub const fn new(records: Vec<HookRecord>) -> Self {
        Self {
            records,
            phase: PhantomData,
        }
    }

    /// Read a report back from the contents of `pre-run.json` / `post-run.json`.
    pub fn from_json(value: &Value) -> Result<Self> {
        let Value::Array(items) = value else {
            bail!("{}-run hook results are not a JSON array", P::phase_name());
        };
        let records = items
            .iter()
            .enumerate()
            .map(|(index, item)| HookRecord::from_json(index, item))
            .collect::<Result<_>>()?;
        Ok(Self::new(records))
    }

    #[must_use]
    pub fn records(&self) -> &[HookRecord] {
        &self.records
    }

    /// Records of hooks that failed or errored.
    pub fn failures(&self) -> impl Iterator<Item = &HookRecord> {
        self.records.iter().filter(|r| !r.status.is_success())
    }

    /// Whether any hook in the phase failed or errored.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.failures().next().is_some()
    }

    /// One line per failed hook, e.g. `pre[0] capture-git-repo: <reason>`.
    #[must_use]
    pub fn failure_lines(&self) -> Vec<String> {
        self.failures()
            .map(|r| {
                format!(
                    "{}[{}] {}: {}",
                    P::phase_name(),
                    r.index,
                    r.hook_id,
                    r.status.message().unwrap_or_default()
                )
            })
            .collect()
    }

    /// The JSON array written to `pre-run.json` / `post-run.json`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Array(self.records.iter().map(HookRecord::to_json).collect())
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
    use capsula_core::hook::{PostRun, PreRun};
    use serde::ser::Error as _;

    struct JsonOutput(Value);

    impl Captured for JsonOutput {
        fn serialize_json(&self) -> Result<Value, serde_json::Error> {
            Ok(self.0.clone())
        }
    }

    struct Unserializable;

    impl Captured for Unserializable {
        fn serialize_json(&self) -> Result<Value, serde_json::Error> {
            Err(serde_json::Error::custom("boom"))
        }
    }

    fn boxed(captured: impl Captured + 'static) -> Box<dyn Captured> {
        Box::new(captured)
    }

    fn record(index: usize, status: HookStatus) -> HookRecord {
        let output = match status {
            HookStatus::Errored { .. } => None,
            HookStatus::Succeeded | HookStatus::Failed { .. } => {
                Some(Map::from_iter([("sha".to_string(), json!("abc"))]))
            }
        };
        HookRecord {
            index,
            hook_id: "capture-git-repo".to_string(),
            config: json!({ "allow_dirty": false }),
            output,
            status,
        }
    }

    #[test]
    fn from_run_records_success() {
        let record = HookRecord::from_run(
            0,
            "capture-cwd",
            json!({}),
            Ok(HookOutcome::success(boxed(JsonOutput(
                json!({ "cwd": "/p" }),
            )))),
        );
        assert_eq!(record.status, HookStatus::Succeeded);
        assert_eq!(record.output.unwrap()["cwd"], "/p");
    }

    #[test]
    fn from_run_records_failure_reason() {
        let record = HookRecord::from_run(
            0,
            "capture-git-repo",
            json!({}),
            Ok(HookOutcome::failure(
                boxed(JsonOutput(json!({ "is_dirty": true }))),
                "repository is dirty",
            )),
        );
        assert_eq!(
            record.status,
            HookStatus::Failed {
                reason: "repository is dirty".to_string()
            }
        );
        assert!(record.output.is_some());
    }

    #[test]
    fn from_run_records_operational_error() {
        let record = HookRecord::from_run(1, "notify-slack", json!({}), Err("timeout".to_string()));
        assert_eq!(
            record,
            HookRecord::errored(1, "notify-slack", json!({}), "timeout")
        );
    }

    #[test]
    fn from_run_treats_unserializable_success_as_error() {
        let record = HookRecord::from_run(
            0,
            "capture-cwd",
            json!({}),
            Ok(HookOutcome::success(boxed(Unserializable))),
        );
        assert!(
            matches!(&record.status, HookStatus::Errored { error } if error.contains("boom")),
            "unexpected status: {:?}",
            record.status
        );
        assert_eq!(record.output, None);
    }

    #[test]
    fn from_run_wraps_non_object_output() {
        let record = HookRecord::from_run(
            0,
            "capture-cwd",
            json!({}),
            Ok(HookOutcome::success(boxed(JsonOutput(json!("/p"))))),
        );
        assert_eq!(record.output.unwrap()["value"], "/p");
    }

    #[test]
    fn to_json_attaches_meta_per_status() {
        let ok = record(0, HookStatus::Succeeded).to_json();
        assert_eq!(ok["sha"], "abc");
        assert_eq!(
            ok["__meta"],
            json!({ "id": "capture-git-repo", "config": { "allow_dirty": false }, "success": true })
        );

        let failed = record(
            0,
            HookStatus::Failed {
                reason: "dirty".to_string(),
            },
        )
        .to_json();
        assert_eq!(failed["__meta"]["success"], false);
        assert_eq!(failed["__meta"]["failure_reason"], "dirty");
        assert!(failed["__meta"].get("error").is_none());

        let errored = record(
            0,
            HookStatus::Errored {
                error: "io".to_string(),
            },
        )
        .to_json();
        assert_eq!(errored["__meta"]["success"], false);
        assert_eq!(errored["__meta"]["error"], "io");
        assert!(errored["__meta"].get("failure_reason").is_none());
        assert_eq!(errored.as_object().unwrap().len(), 1);
    }

    #[test]
    fn json_round_trips() {
        for status in [
            HookStatus::Succeeded,
            HookStatus::Failed {
                reason: "dirty".to_string(),
            },
            HookStatus::Errored {
                error: "io".to_string(),
            },
        ] {
            let original = record(3, status);
            let read_back = HookRecord::from_json(3, &original.to_json()).unwrap();
            assert_eq!(read_back, original);
        }
    }

    #[test]
    fn from_json_reads_legacy_policy_violation_as_recorded() {
        // Before hook outcomes, a dirty repository with `allow_dirty = false`
        // was recorded as a successful capture.
        let legacy = json!({
            "sha": "abc",
            "is_dirty": true,
            "__meta": { "id": "capture-git-repo", "config": { "allow_dirty": false }, "success": true }
        });
        let record = HookRecord::from_json(0, &legacy).unwrap();
        assert_eq!(record.status, HookStatus::Succeeded);
    }

    #[test]
    fn from_json_reads_legacy_error_without_id() {
        let legacy = json!({ "__meta": { "config": {}, "success": false, "error": "boom" } });
        let record = HookRecord::from_json(2, &legacy).unwrap();
        assert_eq!(record, HookRecord::errored(2, "unknown", json!({}), "boom"));
    }

    #[test]
    fn from_json_rejects_record_without_meta() {
        let err = HookRecord::from_json(0, &json!({ "sha": "abc" })).unwrap_err();
        assert!(err.to_string().contains("__meta"), "{err}");
    }

    #[test]
    fn phase_report_lists_failures() {
        let report = PhaseReport::<PreRun>::new(vec![
            record(0, HookStatus::Succeeded),
            record(
                1,
                HookStatus::Failed {
                    reason: "dirty".to_string(),
                },
            ),
            record(
                2,
                HookStatus::Errored {
                    error: "io".to_string(),
                },
            ),
        ]);
        assert!(report.is_failed());
        assert_eq!(
            report.failure_lines(),
            vec![
                "pre[1] capture-git-repo: dirty".to_string(),
                "pre[2] capture-git-repo: io".to_string(),
            ]
        );
    }

    #[test]
    fn phase_report_without_failures_is_not_failed() {
        let report = PhaseReport::<PostRun>::new(vec![record(0, HookStatus::Succeeded)]);
        assert!(!report.is_failed());
        assert_eq!(report.failure_lines(), Vec::<String>::new());
    }

    #[test]
    fn phase_report_round_trips() {
        let report = PhaseReport::<PostRun>::new(vec![
            record(0, HookStatus::Succeeded),
            record(
                1,
                HookStatus::Errored {
                    error: "io".to_string(),
                },
            ),
        ]);
        let read_back = PhaseReport::<PostRun>::from_json(&report.to_json()).unwrap();
        assert_eq!(read_back.records(), report.records());
    }
}
