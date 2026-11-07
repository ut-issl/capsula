mod error;
mod hash;
mod project_glob;

use crate::error::FileHookError;
use crate::hash::file_digest_sha256;
use crate::project_glob::ProjectGlob;
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileHookConfig {
    glob: String,
    #[serde(default)]
    mode: CaptureMode,
    #[serde(default)]
    hash: HashAlgorithm,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    #[default]
    Copy,
    Move,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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
    glob: ProjectGlob,
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
    fn serialize_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, serde_json::Error> {
        capsula_core::captured::to_object(self)
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
        let glob = ProjectGlob::parse(&config.glob).map_err(FileHookError::from)?;
        Ok(Self { config, glob })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> CapsulaResult<HookOutcome<Self::Output>> {
        let artifact_dir = params
            .artifact_dir
            .as_deref()
            .ok_or(FileHookError::ArtifactDirMissing)?;
        self.run(metadata, artifact_dir)
            .map(HookOutcome::success)
            .map_err(CapsulaError::from)
    }

    fn needs_artifact_dir(&self) -> bool {
        true
    }
}

impl FileHook {
    fn run(
        &self,
        metadata: &PreparedRun,
        artifact_dir: &Path,
    ) -> Result<FileCaptured, FileHookError> {
        debug!(
            "FileHook: Searching {} for files matching pattern: {}",
            metadata.project_root.display(),
            self.config.glob
        );

        let files: Vec<_> = self
            .glob
            .find_files(&metadata.project_root)?
            .into_iter()
            .map(|path| {
                debug!("FileHook: Processing file: {}", path.display());
                self.capture_file(&path, artifact_dir)
            })
            .collect::<Result<Vec<_>, FileHookError>>()?;

        debug!("FileHook: Captured {} files", files.len());
        Ok(FileCaptured { files })
    }

    fn capture_file(
        &self,
        path: &Path,
        artifact_dir: &Path,
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
                debug!(
                    "FileHook: {:?} file to artifact directory",
                    self.config.mode
                );
                let file_name = path
                    .file_name()
                    .ok_or_else(|| FileHookError::InvalidRunDir {
                        path: path.to_path_buf(),
                    })?
                    .to_os_string();
                let dest_path = artifact_dir.join(file_name);
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
