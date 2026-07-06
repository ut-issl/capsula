mod error;
mod hash;

use crate::error::DirHookError;
use crate::hash::file_digest_sha256;
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DirHookConfig {
    path: PathBuf,
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
    None,
}

#[derive(Debug)]
pub struct DirHook {
    config: DirHookConfig,
}

#[derive(Debug, Clone)]
struct SourceFile {
    path: PathBuf,
    relative_path: PathBuf,
}

#[derive(Debug, Default)]
struct DirectorySnapshot {
    directories: Vec<PathBuf>,
    files: Vec<SourceFile>,
}

#[derive(Debug, Serialize)]
pub struct DirCapturedFile {
    path: PathBuf,
    relative_path: PathBuf,
    captured_path: Option<PathBuf>,
    hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DirCaptured {
    path: PathBuf,
    captured_path: Option<PathBuf>,
    directories: Vec<PathBuf>,
    files: Vec<DirCapturedFile>,
}

impl Captured for DirCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl<P> Hook<P> for DirHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-dir";

    type Config = DirHookConfig;
    type Output = DirCaptured;

    fn from_config(
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: DirHookConfig = serde_json::from_value(config.clone())?;
        Ok(Self { config })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        self.run(metadata, params.artifact_dir.as_deref())
            .map_err(CapsulaError::from)
    }

    fn needs_artifact_dir(&self) -> bool {
        !matches!(self.config.mode, CaptureMode::None)
    }
}

impl DirHook {
    fn run(
        &self,
        metadata: &PreparedRun,
        artifact_dir: Option<&Path>,
    ) -> Result<DirCaptured, DirHookError> {
        let source_dir =
            Self::resolve_existing_directory(&metadata.project_root, &self.config.path)?;
        debug!("DirHook: Capturing directory: {}", source_dir.display());

        let captured_path = self.destination_root(&source_dir, artifact_dir)?;
        let excluded_roots = Self::excluded_roots(&source_dir, metadata, artifact_dir);
        let snapshot = Self::snapshot_directory(&source_dir, &excluded_roots)?;
        let directories = snapshot.directories.clone();
        let files = self.captured_files(&snapshot, captured_path.as_deref())?;

        match (&self.config.mode, captured_path.as_deref()) {
            (CaptureMode::Copy, Some(destination)) => {
                debug!(
                    "DirHook: Copying directory {} to {}",
                    source_dir.display(),
                    destination.display()
                );
                Self::copy_snapshot(&snapshot, destination)?;
            }
            (CaptureMode::Move, Some(destination)) => {
                debug!(
                    "DirHook: Moving directory {} to {}",
                    source_dir.display(),
                    destination.display()
                );
                Self::move_directory(&source_dir, destination)?;
            }
            (CaptureMode::None, None) => {
                debug!("DirHook: Not copying directory because mode is none");
            }
            (CaptureMode::Copy | CaptureMode::Move, None) => {
                return Err(DirHookError::ArtifactDirMissing);
            }
            (CaptureMode::None, Some(_)) => unreachable!("mode none never creates a destination"),
        }

        debug!("DirHook: Captured {} files", files.len());
        Ok(DirCaptured {
            path: source_dir,
            captured_path,
            directories,
            files,
        })
    }

    fn resolve_existing_directory(
        project_root: &Path,
        configured_path: &Path,
    ) -> Result<PathBuf, DirHookError> {
        let path = if configured_path.is_absolute() {
            configured_path.to_path_buf()
        } else {
            project_root.join(configured_path)
        };

        let canonical = path.canonicalize().map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => DirHookError::DirectoryNotFound { path: path.clone() },
            _ => DirHookError::ResolveDirectory {
                path: path.clone(),
                source,
            },
        })?;

        if canonical.is_dir() {
            Ok(canonical)
        } else {
            Err(DirHookError::NotADirectory { path: canonical })
        }
    }

    fn destination_root(
        &self,
        source_dir: &Path,
        artifact_dir: Option<&Path>,
    ) -> Result<Option<PathBuf>, DirHookError> {
        match self.config.mode {
            CaptureMode::Copy | CaptureMode::Move => {
                let artifact_dir = artifact_dir.ok_or(DirHookError::ArtifactDirMissing)?;
                let directory_name =
                    source_dir
                        .file_name()
                        .ok_or_else(|| DirHookError::InvalidDirectoryName {
                            path: source_dir.to_path_buf(),
                        })?;
                Ok(Some(artifact_dir.join(directory_name)))
            }
            CaptureMode::None => Ok(None),
        }
    }

    fn excluded_roots(
        source_dir: &Path,
        metadata: &PreparedRun,
        artifact_dir: Option<&Path>,
    ) -> Vec<PathBuf> {
        [Some(metadata.run_dir.as_path()), artifact_dir]
            .into_iter()
            .flatten()
            .filter_map(|path| path.canonicalize().ok())
            .filter(|path| path.starts_with(source_dir) && path != source_dir)
            .collect()
    }

    fn snapshot_directory(
        source_dir: &Path,
        excluded_roots: &[PathBuf],
    ) -> Result<DirectorySnapshot, DirHookError> {
        let mut snapshot = DirectorySnapshot::default();
        Self::snapshot_directory_inner(source_dir, source_dir, excluded_roots, &mut snapshot)?;
        snapshot.directories.sort();
        snapshot
            .files
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(snapshot)
    }

    fn snapshot_directory_inner(
        root: &Path,
        current: &Path,
        excluded_roots: &[PathBuf],
        snapshot: &mut DirectorySnapshot,
    ) -> Result<(), DirHookError> {
        let mut entries = std::fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::path);

        entries.into_iter().try_for_each(|entry| {
            let path = entry.path();
            if excluded_roots
                .iter()
                .any(|excluded| path.starts_with(excluded))
            {
                debug!("DirHook: Skipping excluded path: {}", path.display());
                return Ok(());
            }

            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let relative_path = path
                    .strip_prefix(root)
                    .map_err(|_| DirHookError::StripPrefix {
                        path: path.clone(),
                        base: root.to_path_buf(),
                    })?
                    .to_path_buf();
                snapshot.directories.push(relative_path);
                Self::snapshot_directory_inner(root, &path, excluded_roots, snapshot)
            } else if file_type.is_file() {
                let relative_path = path
                    .strip_prefix(root)
                    .map_err(|_| DirHookError::StripPrefix {
                        path: path.clone(),
                        base: root.to_path_buf(),
                    })?
                    .to_path_buf();
                snapshot.files.push(SourceFile {
                    path,
                    relative_path,
                });
                Ok(())
            } else {
                debug!("DirHook: Skipping special path: {}", path.display());
                Ok(())
            }
        })
    }

    fn captured_files(
        &self,
        snapshot: &DirectorySnapshot,
        destination_root: Option<&Path>,
    ) -> Result<Vec<DirCapturedFile>, DirHookError> {
        snapshot
            .files
            .iter()
            .map(|file| {
                let hash = match self.config.hash {
                    HashAlgorithm::Sha256 => {
                        debug!(
                            "DirHook: Computing SHA256 hash for: {}",
                            file.path.display()
                        );
                        Some(format!("sha256:{}", file_digest_sha256(&file.path)?))
                    }
                    HashAlgorithm::None => None,
                };
                let captured_path = destination_root.map(|root| root.join(&file.relative_path));
                Ok(DirCapturedFile {
                    path: file.path.clone(),
                    relative_path: file.relative_path.clone(),
                    captured_path,
                    hash,
                })
            })
            .collect()
    }

    fn copy_snapshot(
        snapshot: &DirectorySnapshot,
        destination_root: &Path,
    ) -> Result<(), DirHookError> {
        std::fs::create_dir_all(destination_root)?;
        snapshot.directories.iter().try_for_each(|relative_path| {
            std::fs::create_dir_all(destination_root.join(relative_path))
        })?;
        snapshot.files.iter().try_for_each(|file| {
            let destination = destination_root.join(&file.relative_path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&file.path, destination)
                .map(|_| ())
                .map_err(DirHookError::from)
        })
    }

    fn move_directory(source_dir: &Path, destination: &Path) -> Result<(), DirHookError> {
        if destination.starts_with(source_dir) {
            return Err(DirHookError::DestinationInsideSource {
                source_dir: source_dir.to_path_buf(),
                destination: destination.to_path_buf(),
            });
        }
        if destination.exists() {
            return Err(DirHookError::DestinationAlreadyExists {
                path: destination.to_path_buf(),
            });
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(source_dir, destination)?;
        Ok(())
    }
}
