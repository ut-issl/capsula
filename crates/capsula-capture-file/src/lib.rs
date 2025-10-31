mod config;
mod error;
mod hash;
use crate::config::{FileHookConfig, FileHookFactory};
use crate::hash::file_digest_sha256;
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, HookFactory, RuntimeParams};

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

#[derive(Debug)]
pub struct FileCapturedPerFile {
    pub path: PathBuf,
    pub copied_path: Option<PathBuf>,
    pub hash: Option<String>,
}

#[derive(Debug)]
pub struct FileCaptured {
    pub files: Vec<FileCapturedPerFile>,
}

impl Captured for FileCaptured {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "files": self.files.iter().map(|f| {
                serde_json::json!({
                    "path": f.path.to_string_lossy(),
                    "copied_path": f.copied_path.as_ref().map(|p| p.to_string_lossy()),
                    "hash": f.hash,
                })
            }).collect::<Vec<_>>(),
        })
    }
}

impl Hook for FileHook {
    type Config = FileHookConfig;
    type Output = FileCaptured;

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(&self, params: &RuntimeParams) -> CapsulaResult<Self::Output> {
        self.run(params).map_err(CapsulaError::from)
    }
}

impl FileHook {
    fn run(&self, params: &RuntimeParams) -> Result<FileCaptured, FileHookError> {
        GlobWalkerBuilder::from_patterns(&params.project_root, &[&self.glob])
            .max_depth(1)
            .build()?
            .filter_map(Result::ok)
            .map(|entry| self.capture_file(entry.path(), params))
            .collect::<Result<Vec<_>, FileHookError>>()
            .map(|files| FileCaptured { files })
    }
    fn capture_file(
        &self,
        path: &Path,
        runtime_params: &RuntimeParams,
    ) -> Result<FileCapturedPerFile, FileHookError> {
        // Compute hash if needed
        let hash = match self.hash {
            HashAlgorithm::Sha256 => Some(format!("sha256:{}", file_digest_sha256(path)?)),
            HashAlgorithm::None => None,
        };

        // Copy or move file if needed
        let copied_path = match self.mode {
            CaptureMode::Copy | CaptureMode::Move => {
                let run_dir = runtime_params
                    .run_dir
                    .as_ref()
                    .ok_or(FileHookError::RunDirNotSet)?;
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

pub fn create_factory() -> Box<dyn HookFactory> {
    Box::new(FileHookFactory)
}
