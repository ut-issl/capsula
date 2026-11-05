//! Resolution of configured paths against the project root.
//!
//! Hook configurations accept paths that are either absolute or relative to
//! the project root. [`ResolvedProjectPath::resolve`] gives both forms the
//! same meaning:
//!
//! - A relative path is joined to the project root; an absolute path is used
//!   as it is. A Windows path that is neither, such as `C:file` or `\file`,
//!   is rejected, because joining it would discard part of the project root.
//! - The result is canonicalized, so it must exist, and symbolic links are
//!   resolved the same way whichever form the path was written in.
//! - With [`Containment::Enforced`], a path outside the project root is
//!   rejected. With [`Containment::Unrestricted`], it is accepted, and
//!   [`ResolvedProjectPath::is_within_project`] tells whether it is inside.
//!
//! The joining and the containment decision are pure functions. `resolve`
//! only adds the two `canonicalize` calls that need the file system.

use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Whether a resolved path must stay inside the project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Containment {
    /// Accept a path outside the project root.
    #[default]
    Unrestricted,
    /// Reject a path outside the project root.
    Enforced,
}

/// An error while resolving a path against the project root.
#[derive(Debug, Error)]
pub enum ProjectPathError {
    /// The project root cannot be canonicalized.
    #[error("Failed to resolve project root '{}'", project_root.display())]
    ProjectRoot {
        /// The project root as given.
        project_root: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The path is relative to a Windows drive or drive root, such as `C:file`
    /// or `\file`, so it is neither absolute nor relative to the project root.
    #[error(
        "Path '{}' is relative to a drive or drive root; write it relative to the project root or as a full absolute path",
        input.display()
    )]
    PartiallyQualified {
        /// The path as configured.
        input: PathBuf,
    },

    /// The path cannot be canonicalized, for example because it does not exist.
    #[error("Failed to resolve path '{}' as '{}'", input.display(), joined.display())]
    Path {
        /// The path as configured.
        input: PathBuf,
        /// The path after joining it to the project root.
        joined: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The path resolves outside the project root while containment is enforced.
    #[error(
        "Path '{}' resolves to '{}', outside the project root '{}'",
        input.display(),
        resolved.display(),
        project_root.display()
    )]
    OutsideProjectRoot {
        /// The path as configured.
        input: PathBuf,
        /// The canonical path.
        resolved: PathBuf,
        /// The canonical project root.
        project_root: PathBuf,
    },
}

/// A canonical path resolved against the project root.
///
/// The only constructor is [`ResolvedProjectPath::resolve`], so the path is
/// always canonical and has passed the requested [`Containment`] check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProjectPath {
    path: PathBuf,
    within_project: bool,
}

impl ResolvedProjectPath {
    /// Resolve `path` against `project_root`.
    ///
    /// # Errors
    ///
    /// Returns an error if the project root or the path cannot be
    /// canonicalized, or if the path is outside the project root while
    /// `containment` is [`Containment::Enforced`].
    pub fn resolve(
        path: &Path,
        project_root: &Path,
        containment: Containment,
    ) -> Result<Self, ProjectPathError> {
        let canonical_root =
            project_root
                .canonicalize()
                .map_err(|source| ProjectPathError::ProjectRoot {
                    project_root: project_root.to_path_buf(),
                    source,
                })?;
        let joined = join_to_root(path, &canonical_root)?;
        let canonical_path = joined
            .canonicalize()
            .map_err(|source| ProjectPathError::Path {
                input: path.to_path_buf(),
                joined,
                source,
            })?;
        check_containment(path, canonical_path, canonical_root, containment)
    }

    /// The canonical path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Convert into the canonical path.
    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }

    /// Whether the path is inside the project root.
    #[must_use]
    pub const fn is_within_project(&self) -> bool {
        self.within_project
    }
}

/// How a path relates to the directory it is resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathForm {
    /// Independent of any base directory, such as `/etc` or `C:\etc`.
    Absolute,
    /// Relative to the base directory, such as `sub/dir`.
    Relative,
    /// Relative to a Windows drive or drive root, such as `C:etc` or `\etc`.
    /// Joining it to a base directory discards all or part of the base.
    PartiallyQualified,
}

impl PathForm {
    fn of(path: &Path) -> Self {
        Self::from_parts(
            path.is_absolute(),
            path.has_root(),
            matches!(path.components().next(), Some(Component::Prefix(_))),
        )
    }

    /// Classify a path from its platform-specific properties. On Unix a path
    /// has no prefix and has a root exactly when it is absolute, so only
    /// Windows paths can be partially qualified.
    const fn from_parts(is_absolute: bool, has_root: bool, has_prefix: bool) -> Self {
        match (is_absolute, has_root || has_prefix) {
            (true, _) => Self::Absolute,
            (false, false) => Self::Relative,
            (false, true) => Self::PartiallyQualified,
        }
    }
}

/// Join `path` to `project_root` unless it is already absolute.
fn join_to_root(path: &Path, project_root: &Path) -> Result<PathBuf, ProjectPathError> {
    match PathForm::of(path) {
        PathForm::Absolute => Ok(path.to_path_buf()),
        PathForm::Relative => Ok(project_root.join(path)),
        PathForm::PartiallyQualified => Err(ProjectPathError::PartiallyQualified {
            input: path.to_path_buf(),
        }),
    }
}

/// Decide whether a canonical path is accepted under `containment`.
///
/// Both paths must already be canonical, so that the component-wise prefix
/// comparison cannot be fooled by `..` or symbolic links.
fn check_containment(
    input: &Path,
    canonical_path: PathBuf,
    canonical_root: PathBuf,
    containment: Containment,
) -> Result<ResolvedProjectPath, ProjectPathError> {
    let within_project = canonical_path.starts_with(&canonical_root);
    match (within_project, containment) {
        (false, Containment::Enforced) => Err(ProjectPathError::OutsideProjectRoot {
            input: input.to_path_buf(),
            resolved: canonical_path,
            project_root: canonical_root,
        }),
        _ => Ok(ResolvedProjectPath {
            path: canonical_path,
            within_project,
        }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "Tests use unwrap/expect for brevity"
    )]

    use super::*;
    use tempfile::TempDir;

    // Pure core.

    #[test]
    fn join_to_root_joins_relative_and_keeps_absolute() {
        let root = Path::new("/project");
        // `/absolute/path` is partially qualified on Windows, so use a drive.
        let absolute = if cfg!(windows) {
            r"C:\absolute\path"
        } else {
            "/absolute/path"
        };
        let cases = [
            (".", "/project/."),
            ("sub/dir", "/project/sub/dir"),
            ("../outside", "/project/../outside"),
            (absolute, absolute),
        ];
        for (input, expected) in cases {
            assert_eq!(
                join_to_root(Path::new(input), root).unwrap(),
                PathBuf::from(expected),
                "input: {input}"
            );
        }
    }

    #[test]
    fn path_form_rejects_only_partially_qualified_paths() {
        // (is_absolute, has_root, has_prefix, expected)
        let cases = [
            // `/etc` on Unix, `C:\etc` or `\\server\share\etc` on Windows.
            (true, true, false, PathForm::Absolute),
            (true, true, true, PathForm::Absolute),
            // `sub/dir`.
            (false, false, false, PathForm::Relative),
            // `\etc` on Windows.
            (false, true, false, PathForm::PartiallyQualified),
            // `C:etc` on Windows.
            (false, false, true, PathForm::PartiallyQualified),
        ];
        for (is_absolute, has_root, has_prefix, expected) in cases {
            assert_eq!(
                PathForm::from_parts(is_absolute, has_root, has_prefix),
                expected,
                "is_absolute: {is_absolute}, has_root: {has_root}, has_prefix: {has_prefix}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn join_to_root_rejects_partially_qualified_windows_paths() {
        let root = Path::new(r"C:\project");
        for input in [r"C:config.json", r"\config.json"] {
            assert!(
                matches!(
                    join_to_root(Path::new(input), root),
                    Err(ProjectPathError::PartiallyQualified { .. })
                ),
                "input: {input}"
            );
        }
        assert_eq!(
            join_to_root(Path::new(r"D:\data"), root).unwrap(),
            PathBuf::from(r"D:\data")
        );
    }

    #[test]
    fn check_containment_applies_policy() {
        let root = PathBuf::from("/project");
        let cases = [
            // (canonical path, containment, expected within_project or None for rejection)
            ("/project", Containment::Unrestricted, Some(true)),
            ("/project", Containment::Enforced, Some(true)),
            ("/project/sub", Containment::Enforced, Some(true)),
            ("/elsewhere", Containment::Unrestricted, Some(false)),
            ("/elsewhere", Containment::Enforced, None),
            // A shared string prefix is not a shared path prefix.
            ("/project-other", Containment::Unrestricted, Some(false)),
            ("/project-other", Containment::Enforced, None),
        ];
        for (path, containment, expected) in cases {
            let result = check_containment(
                Path::new("input"),
                PathBuf::from(path),
                root.clone(),
                containment,
            );
            match expected {
                Some(within) => {
                    let resolved = result.unwrap();
                    assert_eq!(resolved.as_path(), Path::new(path));
                    assert_eq!(resolved.is_within_project(), within, "path: {path}");
                }
                None => assert!(
                    matches!(result, Err(ProjectPathError::OutsideProjectRoot { .. })),
                    "path: {path}, {containment:?}"
                ),
            }
        }
    }

    // Imperative shell.

    /// A temporary `project/` with a `project/sub/` directory and a sibling
    /// `outside/` directory.
    fn fixture() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let project = root.join("project");
        std::fs::create_dir_all(project.join("sub")).unwrap();
        std::fs::create_dir(root.join("outside")).unwrap();
        (tmp, project, root.join("outside"))
    }

    #[test]
    fn resolve_relative_and_absolute_alike() {
        let (_tmp, project, _) = fixture();
        let expected = project.join("sub");
        for input in [Path::new("sub"), Path::new("./sub/../sub"), &expected] {
            let resolved =
                ResolvedProjectPath::resolve(input, &project, Containment::Enforced).unwrap();
            assert_eq!(resolved.as_path(), expected, "input: {}", input.display());
            assert!(resolved.is_within_project());
        }
    }

    #[test]
    fn resolve_outside_depends_on_containment() {
        let (_tmp, project, outside) = fixture();
        for input in [Path::new("../outside"), &outside] {
            let resolved =
                ResolvedProjectPath::resolve(input, &project, Containment::Unrestricted).unwrap();
            assert_eq!(resolved.as_path(), outside);
            assert!(!resolved.is_within_project());

            let result = ResolvedProjectPath::resolve(input, &project, Containment::Enforced);
            assert!(matches!(
                result,
                Err(ProjectPathError::OutsideProjectRoot { .. })
            ));
        }
    }

    #[test]
    fn resolve_missing_path_fails() {
        let (_tmp, project, _) = fixture();
        let result =
            ResolvedProjectPath::resolve(Path::new("missing"), &project, Containment::default());
        assert!(matches!(result, Err(ProjectPathError::Path { .. })));
    }

    #[test]
    fn resolve_missing_project_root_fails() {
        let (_tmp, project, _) = fixture();
        let result = ResolvedProjectPath::resolve(
            Path::new("."),
            &project.join("missing"),
            Containment::default(),
        );
        assert!(matches!(result, Err(ProjectPathError::ProjectRoot { .. })));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_follows_symlinks_for_both_forms() {
        let (_tmp, project, outside) = fixture();
        let link = project.join("link");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        for input in [Path::new("link"), &link] {
            let resolved =
                ResolvedProjectPath::resolve(input, &project, Containment::Unrestricted).unwrap();
            assert_eq!(resolved.as_path(), outside, "input: {}", input.display());
            assert!(!resolved.is_within_project());

            let result = ResolvedProjectPath::resolve(input, &project, Containment::Enforced);
            assert!(matches!(
                result,
                Err(ProjectPathError::OutsideProjectRoot { .. })
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn resolve_canonicalizes_symlinked_project_root() {
        let (tmp, project, _) = fixture();
        let root_link = tmp.path().join("project-link");
        std::os::unix::fs::symlink(&project, &root_link).unwrap();

        let resolved =
            ResolvedProjectPath::resolve(Path::new("sub"), &root_link, Containment::Enforced)
                .unwrap();
        assert_eq!(resolved.as_path(), project.join("sub"));
        assert!(resolved.is_within_project());
    }
}
