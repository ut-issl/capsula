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
}

impl Captured for GitCaptured {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "type": KEY.to_string(),
            "name": self.name,
            "working_dir": self.working_dir.to_string_lossy(),
            "sha": self.sha
        })
    }
}

impl Context for GitContext {
    type Output = GitCaptured;

    fn run(&self, _params: &RuntimeParams) -> CoreResult<Self::Output> {
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

        if !self.allow_dirty {
            let statuses = repo.statuses(None).map_err(GitContextError::from)?;
            if !statuses.is_empty() {
                return Err(GitContextError::DirtyRepository.into());
            }
        }

        Ok(GitCaptured {
            name: self.name.clone(),
            working_dir: repo_path,
            sha: oid.to_string(),
        })
    }
}

/// Create a factory for GitContext
pub fn create_factory() -> Box<dyn ContextFactory> {
    Box::new(GitContextFactory)
}
