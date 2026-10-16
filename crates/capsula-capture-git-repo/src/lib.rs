mod error;

use crate::error::GitHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use tracing::debug;

fn default_remote() -> String {
    "origin".to_string()
}

/// Configuration for `GitHook`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitHookConfig {
    name: String,
    path: PathBuf,
    #[serde(default)]
    allow_dirty: bool,
    #[serde(default)]
    require_pushed: bool,
    #[serde(default = "default_remote")]
    remote: String,
    /// If true, create a lightweight tag `capsula/<run-name>` at the current HEAD
    /// to prevent Git from garbage-collecting the commit.
    #[serde(default)]
    tag_head: bool,
}

#[derive(Debug)]
pub struct GitHook {
    config: GitHookConfig,
    project_root: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct GitCaptured {
    working_dir: PathBuf,
    sha: String, // TODO: Use more suitable type
    is_dirty: bool,
    is_pushed: bool,
    tag: Option<String>,
}

impl Captured for GitCaptured {
    fn serialize_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, serde_json::Error> {
        capsula_core::captured::to_object(self)
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
        let config: GitHookConfig = serde_json::from_value(config.clone())?;

        // `path` is resolved when the hook runs, so that building the hook
        // does not depend on the file system.
        Ok(Self {
            config,
            project_root: project_root.to_path_buf(),
        })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn needs_artifact_dir(&self) -> bool {
        true
    }

    #[tracing::instrument]
    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> CapsulaResult<HookOutcome<Self::Output>> {
        let repo_path = capsula_core::util::resolve_relative(&self.config.path, &self.project_root)
            .map_err(|source| GitHookError::InvalidPath {
                path: self.config.path.clone(),
                source,
            })?;

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

        // Output diff content if dirty
        if is_dirty {
            debug!("GitHook: Generating diff patch for dirty repository");
            let artifact_dir = params
                .artifact_dir
                .as_deref()
                .ok_or(GitHookError::ArtifactDirMissing)?;
            let diff_content = Self::diff_content(&repo)?;
            // Output to a patch file in the artifact directory
            let patch_file_path = artifact_dir.join(format!("{}.patch", self.config.name));
            debug!("GitHook: Writing patch to: {}", patch_file_path.display());
            std::fs::write(&patch_file_path, diff_content).map_err(GitHookError::IoError)?;
        }

        // Check if HEAD is pushed to the configured remote
        debug!(
            "GitHook: Checking if HEAD is pushed to remote '{}'",
            self.config.remote
        );
        let is_pushed = Self::check_pushed(&repo, oid, &self.config.remote)?;
        debug!("GitHook: HEAD is pushed: {}", is_pushed);

        // A policy violation is returned as a failed outcome rather than an
        // error, so that the captured state is still recorded.
        let failure_reasons = [
            (is_dirty && !self.config.allow_dirty)
                .then(|| "repository has uncommitted changes".to_string()),
            (self.config.require_pushed && !is_pushed).then(|| {
                format!(
                    "HEAD commit is not pushed to remote '{}'",
                    self.config.remote
                )
            }),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        // Tag the HEAD commit to prevent garbage collection. A failed pre-run
        // hook stops the run, so there is no run whose commit the tag would keep.
        let stops_run = P::BEFORE_COMMAND && !failure_reasons.is_empty();
        let tag = match (self.config.tag_head, stops_run) {
            (true, false) => Some(Self::tag_head(&repo, oid, &metadata.name)?),
            (true, true) => {
                debug!("GitHook: Not creating a tag because the run will be stopped");
                None
            }
            (false, _) => None,
        };

        let captured = GitCaptured {
            working_dir: repo_path,
            sha: oid.to_string(),
            is_dirty,
            is_pushed,
            tag,
        };

        match failure_reasons.as_slice() {
            [] => Ok(HookOutcome::success(captured)),
            reasons => Ok(HookOutcome::failure(captured, reasons.join("; "))),
        }
    }
}

impl GitHook {
    /// Create a lightweight tag `capsula/<run-name>` at `oid`.
    fn tag_head(repo: &Repository, oid: git2::Oid, run_name: &str) -> CapsulaResult<String> {
        let tag_name = format!("capsula/{run_name}");
        debug!("GitHook: Creating lightweight tag '{}'", tag_name);
        let commit = repo
            .find_object(oid, Some(git2::ObjectType::Commit))
            .map_err(GitHookError::from)?;
        repo.tag_lightweight(&tag_name, &commit, false)
            .map_err(GitHookError::from)?;
        debug!("GitHook: Tag '{}' created successfully", tag_name);
        Ok(tag_name)
    }

    fn check_pushed(repo: &Repository, head_oid: git2::Oid, remote: &str) -> CapsulaResult<bool> {
        let remote_branch_prefix = format!("refs/remotes/{remote}/");

        // Check remote branches: HEAD is at tip or ancestor of a remote branch
        for reference in repo.references().map_err(GitHookError::from)? {
            let reference = reference.map_err(GitHookError::from)?;
            if let Ok(name) = reference.name()
                && name.starts_with(&remote_branch_prefix)
                && let Ok(remote_commit) = reference.peel_to_commit()
                && (remote_commit.id() == head_oid
                    || repo
                        .graph_descendant_of(remote_commit.id(), head_oid)
                        .map_err(GitHookError::from)?)
            {
                debug!("HEAD ({head_oid}) found in remote branch: {name}");
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn diff_content(repo: &Repository) -> CapsulaResult<String> {
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.include_untracked(true);
        let diff = repo
            .diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(GitHookError::from)?;

        let mut diff_content = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            // Non-UTF-8 bytes (e.g. binary blobs in a diff) become
            // replacement characters rather than being silently dropped.
            diff_content.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(GitHookError::from)?;

        Ok(diff_content)
    }
}
