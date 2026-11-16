mod artifact;
mod error;
mod hash;
mod project_glob;
mod source;

use crate::artifact::ArtifactDir;
use crate::error::FileHookError;
use crate::hash::copy_with_digest_sha256;
use crate::project_glob::ProjectGlob;
use crate::source::{FileState, open_source, remove_if_unchanged};
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::fs::File;
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
        self.run(metadata, artifact_dir).map_err(CapsulaError::from)
    }

    fn needs_artifact_dir(&self) -> bool {
        true
    }
}

impl FileHook {
    /// Find the files that match the glob under the project root and capture
    /// each of them.
    ///
    /// An error before any file is captured is returned as `Err`. Once
    /// capturing has started, an error stops it and fails the hook with the
    /// files captured so far as output, so that every file already copied or
    /// moved stays recorded.
    fn run(
        &self,
        metadata: &PreparedRun,
        artifact_dir: &Path,
    ) -> Result<HookOutcome<FileCaptured>, FileHookError> {
        let project_root = &metadata.project_root;
        debug!(
            "FileHook: Searching {} for files matching pattern: {}",
            project_root.display(),
            self.config.glob
        );

        // Find every match, and check every destination, before capturing
        // any file, so that an existing destination fails the hook before a
        // file is copied or moved.
        let matches = self.glob.find_files(project_root)?;
        let artifacts = match self.config.mode {
            CaptureMode::None => None,
            CaptureMode::Copy | CaptureMode::Move => {
                let artifacts = ArtifactDir::new(artifact_dir);
                for relative_path in &matches {
                    if artifacts.exists(relative_path)? {
                        return Err(FileHookError::ArtifactExists {
                            path: artifacts.path_of(relative_path),
                        });
                    }
                }
                Some(artifacts)
            }
        };

        let mut files = Vec::with_capacity(matches.len());
        for relative_path in &matches {
            debug!("FileHook: Processing file: {}", relative_path.display());
            let error = match self.capture_file(
                &project_root.join(relative_path),
                artifacts.as_ref(),
                relative_path,
            ) {
                Ok((file, None)) => {
                    files.push(file);
                    continue;
                }
                Ok((file, Some(error))) => {
                    files.push(file);
                    error
                }
                Err(error) => error,
            };
            return Ok(HookOutcome::failure(
                FileCaptured { files },
                error_chain(&error),
            ));
        }

        debug!("FileHook: Captured {} files", files.len());
        Ok(HookOutcome::success(FileCaptured { files }))
    }

    /// Capture one matched file at `path`: hash it, and copy or move it into
    /// the artifact directory under `relative_path`, its path relative to the
    /// project root.
    ///
    /// If the file was copied but, in move mode, its source could not be
    /// removed, the captured file is returned together with that error, so
    /// that the copy stays recorded.
    fn capture_file(
        &self,
        path: &Path,
        artifacts: Option<&ArtifactDir>,
        relative_path: &Path,
    ) -> Result<(FileCapturedPerFile, Option<FileHookError>), FileHookError> {
        // Read the source only through this handle, so that the hash and the
        // copy describe the same file.
        let mut source = open_source(path)?;
        let Some(artifacts) = artifacts else {
            let hash = self.hash(&mut source)?;
            return Ok((
                FileCapturedPerFile {
                    path: path.to_path_buf(),
                    copied_path: None,
                    hash,
                },
                None,
            ));
        };

        let opened_state = FileState::of(&source.metadata()?);
        let copied_path = artifacts.path_of(relative_path);
        debug!(
            "FileHook: {:?} {} to {}",
            self.config.mode,
            path.display(),
            copied_path.display()
        );
        let (hash, copied_len) = self.copy_to_artifact(&mut source, artifacts, relative_path)?;
        let removal_error = match self.config.mode {
            CaptureMode::Move => {
                remove_if_unchanged(path, &source, opened_state.archived(copied_len)).err()
            }
            CaptureMode::Copy | CaptureMode::None => None,
        };
        Ok((
            FileCapturedPerFile {
                path: path.to_path_buf(),
                copied_path: Some(copied_path),
                hash,
            },
            removal_error,
        ))
    }

    /// Hash the rest of `source` with the configured algorithm.
    fn hash(&self, source: &mut File) -> Result<Option<String>, FileHookError> {
        match self.config.hash {
            HashAlgorithm::Sha256 => {
                let digest = copy_with_digest_sha256(source, &mut std::io::sink())?;
                Ok(Some(format!("sha256:{digest}")))
            }
            HashAlgorithm::None => Ok(None),
        }
    }

    /// Copy the rest of `source_file` to a new artifact at `relative_path`,
    /// hashing the bytes as they are written, so that the hash describes the
    /// archived copy even if the source changes meanwhile. Returns the hash
    /// and the number of bytes copied. An existing artifact is never
    /// overwritten, and a partial copy is removed.
    fn copy_to_artifact(
        &self,
        source_file: &mut File,
        artifacts: &ArtifactDir,
        relative_path: &Path,
    ) -> Result<(Option<String>, u64), FileHookError> {
        let permissions = source_file.metadata()?.permissions();
        let mut destination_file = artifacts.create_new(relative_path)?;

        let result = match self.config.hash {
            HashAlgorithm::Sha256 => copy_with_digest_sha256(source_file, &mut destination_file)
                .map(|digest| Some(format!("sha256:{digest}"))),
            HashAlgorithm::None => std::io::copy(source_file, &mut destination_file).map(|_| None),
        }
        .and_then(|hash| {
            destination_file.set_permissions(permissions)?;
            Ok((hash, destination_file.metadata()?.len()))
        });
        drop(destination_file);

        result.map_err(|error| {
            artifacts.remove_partial(relative_path);
            FileHookError::Io(error)
        })
    }
}

/// Render an error with its sources, for a hook failure reason.
fn error_chain(error: &dyn std::error::Error) -> String {
    std::iter::successors(Some(error), |error| error.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}
