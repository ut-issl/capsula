mod config;
mod error;

use crate::error::GitHookError;

use crate::config::GitHookConfig;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use git2::Repository;
use serde::Serialize;
use std::path::PathBuf;

pub const KEY: &str = "capture-git-repo";

#[derive(Debug)]
pub struct GitHook {
    pub config: GitHookConfig,
    pub name: String,
    pub working_dir: PathBuf,
    pub allow_dirty: bool,
}

#[derive(Debug, Serialize)]
pub struct GitCaptured {
    pub name: String,
    pub working_dir: PathBuf,
    pub sha: String, // TODO: Use more suitable type
    pub is_dirty: bool,
    pub abort_on_dirty: bool,
}

impl Captured for GitCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn abort_requested(&self) -> bool {
        self.is_dirty && self.abort_on_dirty
    }
}

impl<P> Hook<P> for GitHook
where
    P: PhaseMarker,
{
    const KEY: &'static str = KEY;

    type Config = GitHookConfig;
    type Output = GitCaptured;

    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: GitHookConfig = serde_json::from_value(config.clone()).map_err(|e| {
            capsula_core::error::CapsulaError::Configuration {
                message: format!("Invalid git hook configuration: {}", e),
            }
        })?;

        let working_dir = if config.path.is_absolute() {
            config.path.clone()
        } else {
            project_root.join(&config.path).canonicalize()?
        };

        Ok(GitHook {
            name: config.name.clone(),
            allow_dirty: config.allow_dirty,
            working_dir,
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
        let repo_path = if self.working_dir.as_os_str().is_empty() {
            std::env::current_dir()?
        } else {
            self.working_dir.clone()
        };

        let repo = Repository::discover(&repo_path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitHookError::NotARepository
            } else {
                GitHookError::GitOperation(e)
            }
        })?;

        let head = repo.head().map_err(GitHookError::from)?;
        let oid = head.target().ok_or_else(|| GitHookError::HeadNotFound {
            message: "HEAD does not point to a valid commit".to_string(),
        })?;

        // Check if repository is dirty
        let statuses = repo.statuses(None).map_err(GitHookError::from)?;
        let is_dirty = !statuses.is_empty();

        // If dirty and not allowed, we'll signal abort through the Captured trait
        // rather than returning an error, so other hooks can still be captured
        if is_dirty && !self.allow_dirty {
            eprintln!(
                "Warning: Repository has uncommitted changes. Run will be aborted after hooks capture."
            );
        }

        // Output diff content if dirty
        if is_dirty {
            let run_dir = &metadata.run_dir;
            let diff_content = GitHook::diff_content(&repo)?;
            // Output to a patch file in the run directory
            let patch_file_path = run_dir.join(format!("{}.patch", self.name));
            std::fs::write(&patch_file_path, diff_content).map_err(GitHookError::IoError)?;
        }

        Ok(GitCaptured {
            name: self.name.clone(),
            working_dir: repo_path,
            sha: oid.to_string(),
            is_dirty,
            abort_on_dirty: !self.allow_dirty,
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
