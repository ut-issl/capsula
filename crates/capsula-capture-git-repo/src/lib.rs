mod config;
mod error;

use crate::error::GitHookError;

use crate::config::{GitHookConfig, GitHookFactory};
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, RuntimeParams};
use git2::Repository;
use serde_json::json;
use std::path::PathBuf;

pub const KEY: &str = "capture-git-repo";

#[derive(Debug)]
pub struct GitHook {
    pub config: GitHookConfig,
    pub name: String,
    pub working_dir: PathBuf,
    pub allow_dirty: bool,
}

#[derive(Debug)]
pub struct GitCaptured {
    pub name: String,
    pub working_dir: PathBuf,
    pub sha: String, // TODO: Use more suitable type
    pub is_dirty: bool,
    pub abort_on_dirty: bool,
}

impl Captured for GitCaptured {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "id": KEY.to_string(),
            "name": self.name,
            "working_dir": self.working_dir.to_string_lossy(),
            "sha": self.sha,
            "is_dirty": self.is_dirty,
            "abort_on_dirty": self.abort_on_dirty
        })
    }

    fn abort_requested(&self) -> bool {
        self.is_dirty && self.abort_on_dirty
    }
}

impl Hook for GitHook {
    type Config = GitHookConfig;
    type Output = GitCaptured;

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(&self, params: &RuntimeParams) -> CapsulaResult<Self::Output> {
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
            let run_dir =
                params
                    .run_dir
                    .as_ref()
                    .ok_or_else(|| GitHookError::RunDirNotSpecified {
                        message: "Run directory is not specified in runtime parameters".to_string(),
                    })?;
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

/// Create a factory for GitHook
pub fn create_factory() -> Box<dyn HookFactory> {
    Box::new(GitHookFactory)
}
