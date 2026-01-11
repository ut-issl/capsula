mod error;
mod hash;

use crate::error::FileHookError;
use crate::hash::file_digest_sha256;
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct FileHookConfig {
    glob: String,
    #[serde(default)]
    mode: CaptureMode,
    #[serde(default)]
    hash: HashAlgorithm,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    #[default]
    Copy,
    Move,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    #[default]
    Sha256,
    // Md5,
    None,
}

#[derive(Debug)]
pub struct FileHook {
    config: FileHookConfig,
}

#[derive(Debug, Serialize)]
pub struct FileCapturedPerFile {
    path: PathBuf,
    copied_path: Option<PathBuf>,
    hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileCaptured {
    files: Vec<FileCapturedPerFile>,
}

impl Captured for FileCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl<P> Hook<P> for FileHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-file";

    type Config = FileHookConfig;
    type Output = FileCaptured;

    fn from_config(
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: FileHookConfig = serde_json::from_value(config.clone())?;
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
        self.run(metadata).map_err(CapsulaError::from)
    }
}

impl FileHook {
    /// Builds a complete glob pattern from base path and user pattern.
    /// Users must use forward slashes (/) in patterns.
    /// On Windows, this normalizes path separators from backslashes to forward slashes.
    fn build_glob_pattern(base: &Path, pattern: &str) -> String {
        base.join(pattern).to_string_lossy().replace('\\', "/")
    }

    fn run(&self, metadata: &PreparedRun) -> Result<FileCaptured, FileHookError> {
        let pattern = Self::build_glob_pattern(&metadata.project_root, &self.config.glob);
        debug!(
            "FileHook: Searching for files matching pattern: {}",
            pattern
        );

        let files: Vec<_> = glob::glob(&pattern)?
            .filter_map(Result::ok) // Filter out GlobErrors
            .filter(|path| path.is_file()) // Only files, not directories
            .map(|path| {
                debug!("FileHook: Processing file: {}", path.display());
                self.capture_file(&path, &metadata.run_dir)
            })
            .collect::<Result<Vec<_>, FileHookError>>()?;

        debug!("FileHook: Captured {} files", files.len());
        Ok(FileCaptured { files })
    }

    fn capture_file(
        &self,
        path: &Path,
        run_dir: &Path,
    ) -> Result<FileCapturedPerFile, FileHookError> {
        // Compute hash if needed
        let hash = match self.config.hash {
            HashAlgorithm::Sha256 => {
                debug!("FileHook: Computing SHA256 hash for: {}", path.display());
                Some(format!("sha256:{}", file_digest_sha256(path)?))
            }
            HashAlgorithm::None => None,
        };

        // Copy or move file if needed
        let copied_path = match self.config.mode {
            CaptureMode::Copy | CaptureMode::Move => {
                debug!("FileHook: {:?} file to run directory", self.config.mode);
                let file_name = path
                    .file_name()
                    .ok_or_else(|| FileHookError::InvalidRunDir {
                        path: path.to_path_buf(),
                    })?
                    .to_os_string();
                let dest_path = run_dir.join(file_name);
                match self.config.mode {
                    CaptureMode::Copy => std::fs::copy(path, &dest_path).map(|_| ())?,
                    CaptureMode::Move => {
                        std::fs::rename(path, &dest_path)?;
                    }
                    CaptureMode::None => unreachable!(),
                }
                Some(dest_path)
            }
            CaptureMode::None => None,
        };

        Ok(FileCapturedPerFile {
            path: path.to_path_buf(),
            copied_path,
            hash,
        })
    }
}
