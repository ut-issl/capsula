mod error;

use crate::error::GitHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use git2::Repository;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Configuration for `GitHook`
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GitHookConfig {
    name: String,
    path: PathBuf,
    #[serde(default)]
    allow_dirty: bool,
}

#[derive(Debug)]
pub struct GitHook {
    config: GitHookConfig,
    working_dir: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct GitCaptured {
    working_dir: PathBuf,
    sha: String, // TODO: Use more suitable type
    is_dirty: bool,
    #[serde(skip)]
    abort_requested: bool,
}

impl Captured for GitCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn abort_requested(&self) -> bool {
        self.abort_requested
    }
}

impl<P> Hook<P> for GitHook
where
    P: PhaseMarker + std::fmt::Debug,
{
    const ID: &'static str = "capture-git-repo";

    type Config = GitHookConfig;
    type Output = GitCaptured;

    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: GitHookConfig = serde_json::from_value(config.clone()).map_err(|e| {
            capsula_core::error::CapsulaError::Configuration {
                message: format!("Invalid git hook configuration: {e}"),
            }
        })?;

        let working_dir = if config.path.is_absolute() {
            config.path.clone()
        } else {
            project_root.join(&config.path).canonicalize()?
        };

        Ok(Self {
            config,
            working_dir,
        })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    #[tracing::instrument]
    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        let repo_path = if self.working_dir.as_os_str().is_empty() {
            std::env::current_dir()?
        } else {
            self.working_dir.clone()
        };

        debug!(
            "GitHook: Discovering repository at: {}",
            repo_path.display()
        );
        let repo = Repository::discover(&repo_path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitHookError::NotARepository
            } else {
                GitHookError::GitOperation(e)
            }
        })?;

        debug!("GitHook: Getting HEAD commit");
        let head = repo.head().map_err(GitHookError::from)?;
        let oid = head.target().ok_or_else(|| GitHookError::HeadNotFound {
            message: "HEAD does not point to a valid commit".to_string(),
        })?;
        debug!("GitHook: HEAD commit SHA: {}", oid);

        // Check if repository is dirty (excluding ignored files to match git status behavior)
        debug!("GitHook: Checking repository status");
        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true).include_ignored(false);
        let statuses = repo
            .statuses(Some(&mut status_opts))
            .map_err(GitHookError::from)?;
        let is_dirty = !statuses.is_empty();
        debug!("GitHook: Repository is dirty: {}", is_dirty);

        // If dirty and not allowed, we'll signal abort through the Captured trait
        // rather than returning an error, so other hooks can still be captured
        if is_dirty && !self.config.allow_dirty {
            warn!("Repository has uncommitted changes. Run will be aborted after hooks capture.");
        }

        // Output diff content if dirty
        if is_dirty {
            debug!("GitHook: Generating diff patch for dirty repository");
            let run_dir = &metadata.run_dir;
            let diff_content = Self::diff_content(&repo)?;
            // Output to a patch file in the run directory
            let patch_file_path = run_dir.join(format!("{}.patch", self.config.name));
            debug!("GitHook: Writing patch to: {}", patch_file_path.display());
            std::fs::write(&patch_file_path, diff_content).map_err(GitHookError::IoError)?;
        }

        Ok(GitCaptured {
            working_dir: repo_path,
            sha: oid.to_string(),
            is_dirty,
            abort_requested: is_dirty && !self.config.allow_dirty,
        })
    }
}

impl GitHook {
    fn diff_content(repo: &Repository) -> CapsulaResult<String> {
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.include_untracked(true);
        let diff = repo
            .diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(GitHookError::from)?;

        let mut diff_content = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            diff_content.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })
        .map_err(GitHookError::from)?;

        Ok(diff_content)
    }
}
