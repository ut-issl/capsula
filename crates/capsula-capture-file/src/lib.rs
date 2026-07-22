mod error;
mod hash;

use crate::error::FileHookError;
use crate::hash::file_digest_sha256;
use capsula_core::captured::Captured;
use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::project_path::ResolvedProjectPath;
use capsula_core::run::PreparedRun;
use glob::{MatchOptions, Pattern};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use tracing::debug;
use walkdir::WalkDir;

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
struct ProjectRelativeGlob {
    pattern: Pattern,
    max_depth: Option<usize>,
}

impl ProjectRelativeGlob {
    const MATCH_OPTIONS: MatchOptions = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    fn parse(configured: &str) -> Result<Self, FileHookError> {
        let (normalized, component_count, is_recursive) =
            Path::new(configured).components().try_fold(
                (PathBuf::new(), 0, false),
                |(mut normalized, component_count, is_recursive), component| match component {
                    Component::Normal(component) => {
                        normalized.push(component);
                        Ok((
                            normalized,
                            component_count + 1,
                            is_recursive || component == OsStr::new("**"),
                        ))
                    }
                    Component::CurDir => Ok((normalized, component_count, is_recursive)),
                    Component::ParentDir => Err(FileHookError::ParentTraversalPattern {
                        pattern: configured.to_string(),
                    }),
                    Component::Prefix(_) | Component::RootDir => {
                        Err(FileHookError::NonRelativePattern {
                            pattern: configured.to_string(),
                        })
                    }
                },
            )?;

        let mut normalized = normalized.to_string_lossy().into_owned();
        #[cfg(windows)]
        {
            normalized = normalized.replace('\\', "/");
        }
        let has_trailing_separator =
            configured.ends_with('/') || (cfg!(windows) && configured.ends_with('\\'));
        if has_trailing_separator && !normalized.is_empty() {
            normalized.push('/');
        }

        Ok(Self {
            pattern: Pattern::new(&normalized)?,
            max_depth: (!is_recursive).then_some(component_count),
        })
    }

    fn matches(&self, path: &Path) -> bool {
        self.pattern.matches_path_with(path, Self::MATCH_OPTIONS)
    }
}

#[derive(Debug)]
pub struct FileHook {
    config: FileHookConfig,
    project_root: ResolvedProjectPath,
    glob: ProjectRelativeGlob,
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
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: FileHookConfig = serde_json::from_value(config.clone())?;
        let glob = ProjectRelativeGlob::parse(&config.glob)?;
        let project_root = ResolvedProjectPath::resolve_existing(project_root, project_root)
            .map_err(FileHookError::from)?;

        Ok(Self {
            config,
            project_root,
            glob,
        })
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
        _metadata: &PreparedRun,
        artifact_dir: &Path,
    ) -> Result<FileCaptured, FileHookError> {
        debug!(
            "FileHook: Searching under {} for files matching pattern: {}",
            self.project_root.as_path().display(),
            self.config.glob
        );

        // Validate the complete match set before copy or move operations can
        // mutate the project or populate the artifact directory.
        let matching_files = self.matching_files()?;
        let files = matching_files
            .iter()
            .map(|path| {
                debug!("FileHook: Processing file: {}", path.as_path().display());
                self.capture_file(path, artifact_dir)
            })
            .collect::<Result<Vec<_>, FileHookError>>()?;

        debug!("FileHook: Captured {} files", files.len());
        Ok(FileCaptured { files })
    }

    fn matching_files(&self) -> Result<Vec<ResolvedProjectPath>, FileHookError> {
        let Some(max_depth) = self.glob.max_depth else {
            return self.walk_matching_files(WalkDir::new(self.project_root.as_path()));
        };
        if max_depth == 0 {
            return Ok(Vec::new());
        }

        self.walk_matching_files(WalkDir::new(self.project_root.as_path()).max_depth(max_depth))
    }

    fn walk_matching_files(
        &self,
        walker: WalkDir,
    ) -> Result<Vec<ResolvedProjectPath>, FileHookError> {
        let project_root = self.project_root.as_path();

        walker
            .follow_links(false)
            .follow_root_links(false)
            .min_depth(1)
            .sort_by_file_name()
            .into_iter()
            .try_fold(Vec::new(), |mut files, entry| {
                let entry = entry?;
                let path = entry.path();
                let relative_path = path.strip_prefix(project_root).map_err(|_| {
                    FileHookError::WalkedPathOutsideProject {
                        path: path.to_path_buf(),
                        project_root: project_root.to_path_buf(),
                    }
                })?;

                if !self.glob.matches(relative_path) {
                    return Ok(files);
                }

                let file_type = entry.file_type();
                if file_type.is_symlink() {
                    return Err(FileHookError::SymlinkNotAllowed {
                        path: path.to_path_buf(),
                    });
                }
                if file_type.is_file() {
                    files.push(ResolvedProjectPath::resolve_existing(path, project_root)?);
                }

                Ok(files)
            })
    }

    fn revalidate_source(source: &ResolvedProjectPath) -> Result<&Path, FileHookError> {
        let path = source.as_path();
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            return Err(FileHookError::SymlinkNotAllowed {
                path: path.to_path_buf(),
            });
        }
        if !metadata.file_type().is_file() {
            return Err(FileHookError::SourceNotRegularFile {
                path: path.to_path_buf(),
            });
        }

        let current = ResolvedProjectPath::resolve_existing(path, source.project_root())?;
        if &current != source {
            return Err(FileHookError::SourcePathChanged {
                path: path.to_path_buf(),
                resolved: current.into_path_buf(),
            });
        }

        Ok(path)
    }

    fn capture_file(
        &self,
        source: &ResolvedProjectPath,
        artifact_dir: &Path,
    ) -> Result<FileCapturedPerFile, FileHookError> {
        let path = Self::revalidate_source(source)?;

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
