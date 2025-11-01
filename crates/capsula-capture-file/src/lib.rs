mod config;
mod error;
mod hash;
use crate::config::FileHookConfig;
use crate::hash::file_digest_sha256;
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use globwalk::GlobWalkerBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use error::FileHookError;

pub const KEY: &str = "capture-file";

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
    pub config: FileHookConfig,
    pub glob: String,
    pub mode: CaptureMode,
    pub hash: HashAlgorithm,
}

#[derive(Debug, Serialize)]
pub struct FileCapturedPerFile {
    pub path: PathBuf,
    pub copied_path: Option<PathBuf>,
    pub hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileCaptured {
    pub files: Vec<FileCapturedPerFile>,
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
    const KEY: &'static str = KEY;

    type Config = FileHookConfig;
    type Output = FileCaptured;

    fn from_config(
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: FileHookConfig = serde_json::from_value(config.clone())?;

        Ok(FileHook {
            glob: config.glob.clone(),
            mode: config.mode.clone(),
            hash: config.hash.clone(),
            config,
        })
    }

    fn id(&self) -> String {
        KEY.to_string()
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
    fn run(&self, metadata: &PreparedRun) -> Result<FileCaptured, FileHookError> {
        GlobWalkerBuilder::from_patterns(&metadata.project_root, &[&self.glob])
            .max_depth(1)
            .build()?
            .filter_map(Result::ok)
            .map(|entry| self.capture_file(entry.path(), &metadata.run_dir))
            .collect::<Result<Vec<_>, FileHookError>>()
            .map(|files| FileCaptured { files })
    }

    fn capture_file(
        &self,
        path: &Path,
        run_dir: &Path,
    ) -> Result<FileCapturedPerFile, FileHookError> {
        // Compute hash if needed
        let hash = match self.hash {
            HashAlgorithm::Sha256 => Some(format!("sha256:{}", file_digest_sha256(path)?)),
            HashAlgorithm::None => None,
        };

        // Copy or move file if needed
        let copied_path = match self.mode {
            CaptureMode::Copy | CaptureMode::Move => {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| FileHookError::InvalidRunDir {
                        path: path.to_path_buf(),
                    })?
                    .to_os_string();
                let dest_path = run_dir.join(file_name);
                match self.mode {
                    CaptureMode::Copy => std::fs::copy(path, &dest_path).map(|_| ())?,
                    CaptureMode::Move => {
                        std::fs::rename(path, &dest_path)?;
                    }
                    _ => unreachable!(),
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
