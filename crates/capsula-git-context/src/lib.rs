mod config;
mod error;

use crate::error::GitContextError;

use crate::config::GitContextFactory;
use capsula_core::captured::Captured;
use capsula_core::context::{Context, ContextFactory, RuntimeParams};
use capsula_core::error::CoreResult;
use git2::Repository;
use serde_json::json;
use std::path::PathBuf;

pub const KEY: &str = "git";

#[derive(Debug)]
pub struct GitContext {
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
            "type": KEY.to_string(),
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

impl Context for GitContext {
    type Output = GitCaptured;

    fn run(&self, params: &RuntimeParams) -> CoreResult<Self::Output> {
        let repo_path = if self.working_dir.as_os_str().is_empty() {
            std::env::current_dir()?
        } else {
            self.working_dir.clone()
        };

        let repo = Repository::discover(&repo_path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitContextError::NotARepository
            } else {
                GitContextError::GitOperation(e)
            }
        })?;

        let head = repo.head().map_err(GitContextError::from)?;
        let oid = head.target().ok_or_else(|| GitContextError::HeadNotFound {
            message: "HEAD does not point to a valid commit".to_string(),
        })?;

        // Check if repository is dirty
        let statuses = repo.statuses(None).map_err(GitContextError::from)?;
        let is_dirty = !statuses.is_empty();

        // If dirty and not allowed, we'll signal abort through the Captured trait
        // rather than returning an error, so other contexts can still be captured
        if is_dirty && !self.allow_dirty {
            eprintln!(
                "Warning: Repository has uncommitted changes. Run will be aborted after context capture."
            );
        }

        // Output diff content if dirty
        if is_dirty {
            let run_dir =
                params
                    .run_dir
                    .as_ref()
                    .ok_or_else(|| GitContextError::RunDirNotSpecified {
                        message: "Run directory is not specified in runtime parameters".to_string(),
                    })?;
            let diff_content = GitContext::diff_content(&repo)?;
            // Output to a patch file in the run directory
            let patch_file_path = run_dir.join(format!("{}.patch", self.name));
            std::fs::write(&patch_file_path, diff_content).map_err(GitContextError::IoError)?;
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

impl GitContext {
    fn diff_content(repo: &Repository) -> CoreResult<String> {
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.include_untracked(true);
        let diff = repo
            .diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(GitContextError::from)?;

        let mut diff_content = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            diff_content.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })
        .map_err(GitContextError::from)?;

        Ok(diff_content)
    }
}

/// Create a factory for GitContext
pub fn create_factory() -> Box<dyn ContextFactory> {
    Box::new(GitContextFactory)
}
