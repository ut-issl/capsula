//! Project-root-aware path resolution.
//!
//! Capsula configuration commonly accepts paths that may be absolute or
//! relative to the project root. This module exposes an invariant-carrying
//! resolved path type: constructing a [`ResolvedProjectPath`] canonicalizes both
//! the project root and the target path, resolves symlinks consistently, and
//! rejects targets outside the project root.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur while resolving a project-root-aware path.
#[derive(Debug, Error)]
pub enum ProjectPathError {
    /// The configured project root could not be canonicalized.
    #[error("failed to resolve project root '{project_root}': {source}")]
    ProjectRootCanonicalize {
        /// The project root supplied by the caller.
        project_root: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The configured path could not be canonicalized after joining it to the
    /// project root when needed.
    #[error(
        "failed to resolve project path '{input}' as '{resolved}' under project root '{project_root}': {source}"
    )]
    PathCanonicalize {
        /// The original path from configuration.
        input: PathBuf,
        /// The path attempted after applying project-root-relative semantics.
        resolved: PathBuf,
        /// The canonicalized project root.
        project_root: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The canonical target is outside the canonical project root.
    #[error(
        "project path '{input}' resolved to '{resolved}', outside project root '{project_root}'"
    )]
    EscapesProjectRoot {
        /// The original path from configuration.
        input: PathBuf,
        /// The canonical path that escaped the project root.
        resolved: PathBuf,
        /// The canonicalized project root.
        project_root: PathBuf,
    },
}

/// A canonicalized path that has been proven to be inside a canonical project
/// root.
///
/// Fields are private, and the only public constructor is
/// [`ResolvedProjectPath::resolve_existing`], so every value of this type is
/// canonicalized and contained within its canonical project root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedProjectPath {
    path: PathBuf,
    project_root: PathBuf,
}

impl ResolvedProjectPath {
    /// Resolve `path` against `project_root`, canonicalize it, and require the
    /// result to stay inside the canonical project root.
    ///
    /// Relative paths are resolved from `project_root`. Absolute paths are also
    /// canonicalized and must still point inside `project_root`. The target path
    /// must exist because canonicalization resolves symlinks.
    pub fn resolve_existing(path: &Path, project_root: &Path) -> Result<Self, ProjectPathError> {
        let canonical_root = project_root.canonicalize().map_err(|source| {
            ProjectPathError::ProjectRootCanonicalize {
                project_root: project_root.to_path_buf(),
                source,
            }
        })?;
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            canonical_root.join(path)
        };
        let resolved =
            joined
                .canonicalize()
                .map_err(|source| ProjectPathError::PathCanonicalize {
                    input: path.to_path_buf(),
                    resolved: joined,
                    project_root: canonical_root.clone(),
                    source,
                })?;

        if resolved.starts_with(&canonical_root) {
            Ok(Self {
                path: resolved,
                project_root: canonical_root,
            })
        } else {
            Err(ProjectPathError::EscapesProjectRoot {
                input: path.to_path_buf(),
                resolved,
                project_root: canonical_root,
            })
        }
    }

    /// Return the canonicalized path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Return the canonicalized project root used for containment.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Clone the canonicalized path into a [`PathBuf`].
    #[must_use]
    pub fn to_path_buf(&self) -> PathBuf {
        self.path.clone()
    }

    /// Consume this value and return the canonicalized path.
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }
}

impl AsRef<Path> for ResolvedProjectPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectPathError, ResolvedProjectPath};
    use std::fs;
    use std::path::{Path, PathBuf};
    use ulid::Ulid;

    fn temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("capsula_project_path_{name}_{}", Ulid::new()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn resolves_relative_path_inside_project_root() {
        let root = temp_dir("relative");
        let nested = root.join("nested");
        fs::create_dir(&nested).expect("nested dir should be created");

        let resolved = ResolvedProjectPath::resolve_existing(Path::new("nested"), &root)
            .expect("path should resolve");

        assert_eq!(resolved.as_path(), nested.canonicalize().unwrap());
        assert_eq!(resolved.project_root(), root.canonicalize().unwrap());

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn canonicalizes_absolute_path_inside_project_root() {
        let root = temp_dir("absolute");
        let nested = root.join("nested");
        fs::create_dir(&nested).expect("nested dir should be created");

        let resolved = ResolvedProjectPath::resolve_existing(&nested, &root)
            .expect("absolute in-project path should resolve");

        assert_eq!(resolved.as_path(), nested.canonicalize().unwrap());

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rejects_relative_path_that_escapes_project_root() {
        let parent = temp_dir("relative_escape");
        let root = parent.join("project");
        let outside = parent.join("outside");
        fs::create_dir(&root).expect("project dir should be created");
        fs::create_dir(&outside).expect("outside dir should be created");

        let result = ResolvedProjectPath::resolve_existing(Path::new("../outside"), &root);

        assert!(matches!(
            result,
            Err(ProjectPathError::EscapesProjectRoot { .. })
        ));

        fs::remove_dir_all(parent).ok();
    }

    #[test]
    fn rejects_absolute_path_outside_project_root() {
        let parent = temp_dir("absolute_escape");
        let root = parent.join("project");
        let outside = parent.join("outside");
        fs::create_dir(&root).expect("project dir should be created");
        fs::create_dir(&outside).expect("outside dir should be created");

        let result = ResolvedProjectPath::resolve_existing(&outside, &root);

        assert!(matches!(
            result,
            Err(ProjectPathError::EscapesProjectRoot { .. })
        ));

        fs::remove_dir_all(parent).ok();
    }

    #[test]
    fn rejects_missing_path() {
        let root = temp_dir("missing");

        let result = ResolvedProjectPath::resolve_existing(Path::new("missing"), &root);

        assert!(matches!(
            result,
            Err(ProjectPathError::PathCanonicalize { .. })
        ));

        fs::remove_dir_all(root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_that_escapes_project_root() {
        use std::os::unix::fs::symlink;

        let parent = temp_dir("symlink_escape");
        let root = parent.join("project");
        let outside = parent.join("outside");
        fs::create_dir(&root).expect("project dir should be created");
        fs::create_dir(&outside).expect("outside dir should be created");
        symlink(&outside, root.join("link")).expect("symlink should be created");

        let result = ResolvedProjectPath::resolve_existing(Path::new("link"), &root);

        assert!(matches!(
            result,
            Err(ProjectPathError::EscapesProjectRoot { .. })
        ));

        fs::remove_dir_all(parent).ok();
    }
}
