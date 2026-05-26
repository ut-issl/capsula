mod error;

use crate::error::GitHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use gix::Repository;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Write as _};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

fn default_remote() -> String {
    "origin".to_string()
}

/// Configuration for `GitHook`
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    working_dir: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct GitCaptured {
    working_dir: PathBuf,
    sha: String, // TODO: Use more suitable type
    is_dirty: bool,
    is_pushed: bool,
    tag: Option<String>,
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
        let config: GitHookConfig = serde_json::from_value(config.clone())?;

        let working_dir = capsula_core::util::resolve_relative(&config.path, project_root)?;

        Ok(Self {
            config,
            working_dir,
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
        let repo = gix::discover(&repo_path).map_err(|error| match &error {
            gix::discover::Error::Discover(
                gix::discover::upwards::Error::NoGitRepository { .. }
                | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling { .. }
                | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. },
            ) => GitHookError::NotARepository,
            _ => GitHookError::Discover(error),
        })?;

        debug!("GitHook: Getting HEAD commit");
        let oid = repo
            .head_id()
            .map_err(|error| GitHookError::HeadNotFound {
                message: error.to_string(),
            })?
            .detach();
        debug!("GitHook: HEAD commit SHA: {}", oid);

        // Check if repository is dirty (excluding ignored files to match git status behavior)
        debug!("GitHook: Checking repository status");
        let is_dirty = Self::is_dirty(&repo)?;
        debug!("GitHook: Repository is dirty: {}", is_dirty);

        // If dirty and not allowed, we'll signal abort through the Captured trait
        // rather than returning an error, so other hooks can still be captured
        if is_dirty && !self.config.allow_dirty {
            warn!("Repository has uncommitted changes. Run will be aborted after hooks capture.");
        }

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

        if self.config.require_pushed && !is_pushed {
            warn!(
                "HEAD commit is not pushed to remote '{}'. Run will be aborted after hooks capture.",
                self.config.remote
            );
        }

        // Tag the HEAD commit to prevent garbage collection
        let tag = if self.config.tag_head {
            let tag_name = format!("capsula/{}", metadata.name);
            debug!("GitHook: Creating lightweight tag '{}'", tag_name);
            repo.tag_reference(
                &tag_name,
                oid,
                gix::refs::transaction::PreviousValue::MustNotExist,
            )
            .map_err(GitHookError::TagReference)?;
            debug!("GitHook: Tag '{}' created successfully", tag_name);
            Some(tag_name)
        } else {
            None
        };

        Ok(GitCaptured {
            working_dir: repo_path,
            sha: oid.to_string(),
            is_dirty,
            is_pushed,
            tag,
            abort_requested: (is_dirty && !self.config.allow_dirty)
                || (self.config.require_pushed && !is_pushed),
        })
    }
}

impl GitHook {
    fn is_dirty(repo: &Repository) -> CapsulaResult<bool> {
        let mut status = repo
            .status(gix::progress::Discard)
            .map_err(GitHookError::Status)?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_iter(Vec::<gix::bstr::BString>::new())
            .map_err(GitHookError::StatusIterator)?;

        status
            .next()
            .transpose()
            .map(|item| item.is_some())
            .map_err(GitHookError::StatusItem)
            .map_err(Into::into)
    }

    fn check_pushed(
        repo: &Repository,
        head_oid: gix::ObjectId,
        remote: &str,
    ) -> CapsulaResult<bool> {
        let remote_branch_prefix = format!("refs/remotes/{remote}/");

        // Check remote branches: HEAD is at tip or ancestor of a remote branch
        for reference in repo
            .references()
            .map_err(GitHookError::References)?
            .prefixed(remote_branch_prefix.as_str())
            .map_err(GitHookError::ReferenceIterator)?
        {
            let reference = reference.map_err(|source| GitHookError::ReferenceItem {
                message: source.to_string(),
            })?;
            let name = reference.name().as_bstr().to_string();
            let Some(remote_oid) = reference.target().try_id().map(ToOwned::to_owned) else {
                continue;
            };
            if (remote_oid == head_oid || Self::is_ancestor(repo, head_oid, remote_oid)?)
                && name.starts_with(&remote_branch_prefix)
            {
                debug!("HEAD ({head_oid}) found in remote branch: {name}");
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn is_ancestor(
        repo: &Repository,
        ancestor: gix::ObjectId,
        descendant: gix::ObjectId,
    ) -> CapsulaResult<bool> {
        match repo.merge_base(ancestor, descendant) {
            Ok(merge_base) => Ok(merge_base.detach() == ancestor),
            Err(gix::repository::merge_base::Error::NotFound { .. }) => Ok(false),
            Err(error) => Err(GitHookError::MergeBase(error).into()),
        }
    }

    fn diff_content(repo: &Repository) -> CapsulaResult<String> {
        let workdir = repo.workdir().ok_or(GitHookError::WorktreeMissing)?;
        let mut status = repo
            .status(gix::progress::Discard)
            .map_err(GitHookError::Status)?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_iter(Vec::<gix::bstr::BString>::new())
            .map_err(GitHookError::StatusIterator)?;

        let mut diff_content = String::new();
        for item in &mut status {
            Self::append_status_patch(
                repo,
                workdir,
                item.map_err(GitHookError::StatusItem)?,
                &mut diff_content,
            )?;
        }

        Ok(diff_content)
    }

    fn append_status_patch(
        repo: &Repository,
        workdir: &Path,
        item: gix::status::Item,
        diff_content: &mut String,
    ) -> CapsulaResult<()> {
        use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};

        match item {
            gix::status::Item::IndexWorktree(gix::status::index_worktree::Item::Modification {
                entry,
                rela_path: path,
                status,
                ..
            }) => match status {
                EntryStatus::Change(Change::Removed) => {
                    let old = Self::blob_content(repo, entry.id)?;
                    Self::append_file_patch(diff_content, &path, Some(&old), None);
                }
                EntryStatus::Change(
                    Change::Modification { .. }
                    | Change::Type { .. }
                    | Change::SubmoduleModification(_),
                ) => {
                    let old = Self::blob_content(repo, entry.id)?;
                    let new = Self::worktree_content(workdir, &path)?;
                    Self::append_file_patch(diff_content, &path, Some(&old), Some(&new));
                }
                EntryStatus::IntentToAdd => {
                    let new = Self::worktree_content(workdir, &path)?;
                    Self::append_file_patch(diff_content, &path, None, Some(&new));
                }
                EntryStatus::Conflict { .. } | EntryStatus::NeedsUpdate(_) => {}
            },
            gix::status::Item::IndexWorktree(
                gix::status::index_worktree::Item::DirectoryContents { entry, .. },
            ) => {
                let new = Self::worktree_content(workdir, &entry.rela_path)?;
                Self::append_file_patch(diff_content, &entry.rela_path, None, Some(&new));
            }
            gix::status::Item::IndexWorktree(gix::status::index_worktree::Item::Rewrite {
                source,
                dirwalk_entry,
                ..
            }) => {
                let old = match source {
                    gix::status::index_worktree::RewriteSource::RewriteFromIndex {
                        source_entry,
                        ..
                    } => Some(Self::blob_content(repo, source_entry.id)?),
                    gix::status::index_worktree::RewriteSource::CopyFromDirectoryEntry {
                        ..
                    } => None,
                };
                let new = Self::worktree_content(workdir, &dirwalk_entry.rela_path)?;
                Self::append_file_patch(
                    diff_content,
                    &dirwalk_entry.rela_path,
                    old.as_deref(),
                    Some(&new),
                );
            }
            gix::status::Item::TreeIndex(change) => {
                Self::append_index_patch(repo, diff_content, change)?;
            }
        }

        Ok(())
    }

    fn append_index_patch(
        repo: &Repository,
        diff_content: &mut String,
        change: gix::diff::index::Change,
    ) -> CapsulaResult<()> {
        match change {
            gix::diff::index::Change::Addition { location, id, .. } => {
                let new = Self::blob_content(repo, id.into_owned())?;
                Self::append_file_patch(diff_content, &location.into_owned(), None, Some(&new));
            }
            gix::diff::index::Change::Deletion { location, id, .. } => {
                let old = Self::blob_content(repo, id.into_owned())?;
                Self::append_file_patch(diff_content, &location.into_owned(), Some(&old), None);
            }
            gix::diff::index::Change::Modification {
                location,
                previous_id,
                id,
                ..
            } => {
                let old = Self::blob_content(repo, previous_id.into_owned())?;
                let new = Self::blob_content(repo, id.into_owned())?;
                Self::append_file_patch(
                    diff_content,
                    &location.into_owned(),
                    Some(&old),
                    Some(&new),
                );
            }
            gix::diff::index::Change::Rewrite {
                location,
                source_id,
                id,
                ..
            } => {
                let old = Self::blob_content(repo, source_id.into_owned())?;
                let new = Self::blob_content(repo, id.into_owned())?;
                Self::append_file_patch(
                    diff_content,
                    &location.into_owned(),
                    Some(&old),
                    Some(&new),
                );
            }
        }
        Ok(())
    }

    fn blob_content(repo: &Repository, id: gix::ObjectId) -> CapsulaResult<Vec<u8>> {
        repo.find_blob(id)
            .map(|mut blob| blob.take_data())
            .map_err(GitHookError::FindBlob)
            .map_err(Into::into)
    }

    fn worktree_content(workdir: &Path, repo_path: &[u8]) -> CapsulaResult<Vec<u8>> {
        let path = workdir.join(Self::path_for_filesystem(repo_path));
        std::fs::read(path)
            .map_err(GitHookError::IoError)
            .map_err(Into::into)
    }

    fn append_file_patch(
        diff_content: &mut String,
        repo_path: &[u8],
        old: Option<&[u8]>,
        new: Option<&[u8]>,
    ) {
        if old == new {
            return;
        }

        let path = Self::path_for_patch(repo_path);
        let _ = writeln!(diff_content, "diff --git a/{path} b/{path}");
        match (old, new) {
            (None, Some(new)) => {
                diff_content.push_str("new file mode 100644\n");
                Self::append_unified_hunk(
                    diff_content,
                    "/dev/null",
                    &format!("b/{path}"),
                    &[],
                    new,
                );
            }
            (Some(old), None) => {
                diff_content.push_str("deleted file mode 100644\n");
                Self::append_unified_hunk(
                    diff_content,
                    &format!("a/{path}"),
                    "/dev/null",
                    old,
                    &[],
                );
            }
            (Some(old), Some(new)) if Self::is_binary(old) || Self::is_binary(new) => {
                let _ = writeln!(diff_content, "Binary files a/{path} and b/{path} differ");
            }
            (Some(old), Some(new)) => {
                Self::append_unified_hunk(
                    diff_content,
                    &format!("a/{path}"),
                    &format!("b/{path}"),
                    old,
                    new,
                );
            }
            (None, None) => {}
        }
    }

    fn append_unified_hunk(
        diff_content: &mut String,
        old_path: &str,
        new_path: &str,
        old: &[u8],
        new: &[u8],
    ) {
        let _ = writeln!(diff_content, "--- {old_path}");
        let _ = writeln!(diff_content, "+++ {new_path}");
        let _ = writeln!(
            diff_content,
            "@@ -{} +{} @@",
            Self::hunk_range(old),
            Self::hunk_range(new)
        );
        Self::append_prefixed_lines(diff_content, '-', old);
        Self::append_prefixed_lines(diff_content, '+', new);
    }

    fn append_prefixed_lines(diff_content: &mut String, prefix: char, content: &[u8]) {
        if content.is_empty() {
            return;
        }

        for line in content.split_inclusive(|byte| *byte == b'\n') {
            diff_content.push(prefix);
            diff_content.push_str(&String::from_utf8_lossy(line));
            if !line.ends_with(b"\n") {
                diff_content.push('\n');
            }
        }

        if !content.ends_with(b"\n") {
            diff_content.push_str("\\ No newline at end of file\n");
        }
    }

    fn hunk_range(content: &[u8]) -> String {
        if content.is_empty() {
            return "0,0".to_string();
        }
        format!(
            "1,{}",
            content.split_inclusive(|byte| *byte == b'\n').count()
        )
    }

    fn path_for_patch(repo_path: &[u8]) -> String {
        String::from_utf8_lossy(repo_path).into_owned()
    }

    fn path_for_filesystem(repo_path: &[u8]) -> PathBuf {
        PathBuf::from(Self::path_for_patch(repo_path))
    }

    fn is_binary(content: &[u8]) -> bool {
        content.contains(&0)
    }
}
