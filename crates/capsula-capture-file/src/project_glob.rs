//! Glob patterns relative to the project root.
//!
//! [`ProjectGlob::parse`] accepts only a pattern relative to the project root
//! and without a `..` component, so that a pattern cannot name a path outside
//! the project by mistake. Matching is left to [`glob::glob`], which follows
//! symbolic links like a shell does.

use glob::{GlobError, Pattern, PatternError};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// An error while parsing a glob pattern.
#[derive(Debug, Error)]
pub enum ProjectGlobError {
    /// The pattern is absolute or has a root or a Windows path prefix.
    #[error("Glob pattern '{pattern}' must be relative to the project root")]
    NotRelative { pattern: String },

    /// The pattern contains a `..` component.
    #[error("Glob pattern '{pattern}' must not contain a '..' component")]
    ParentTraversal { pattern: String },

    /// The pattern is not a valid glob.
    #[error("Invalid glob pattern '{pattern}'")]
    Invalid {
        pattern: String,
        #[source]
        source: PatternError,
    },
}

/// An error while searching the project for matching files.
#[derive(Debug, Error)]
pub enum FindFilesError {
    /// The project root is not valid UTF-8, so it cannot be put in a glob.
    #[error("Project root '{}' is not valid UTF-8", path.display())]
    NonUtf8Root { path: PathBuf },

    /// The project root makes the glob pattern invalid.
    #[error("Project root '{}' cannot be used in a glob pattern", path.display())]
    InvalidRoot {
        path: PathBuf,
        #[source]
        source: PatternError,
    },

    /// A directory that can contain a match cannot be read.
    #[error(transparent)]
    Read(#[from] GlobError),

    /// A match is not beneath the project root, which `glob` should never
    /// return.
    #[error("Matched path '{}' is not beneath the project root", path.display())]
    OutsideRoot { path: PathBuf },
}

/// A glob pattern relative to the project root.
#[derive(Debug, Clone)]
pub struct ProjectGlob {
    /// A valid glob with `/` separators and without `.` or `..` components.
    pattern: Pattern,
}

impl ProjectGlob {
    /// Parse a glob pattern relative to the project root.
    ///
    /// `.` components are dropped. On Windows, `\` is a path separator;
    /// elsewhere it is an ordinary file name character.
    ///
    /// # Errors
    ///
    /// Returns an error if the pattern is not relative, contains a `..`
    /// component, or is not a valid glob.
    pub fn parse(pattern: &str) -> Result<Self, ProjectGlobError> {
        let components = Path::new(pattern)
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => Some(Ok(name.to_string_lossy())),
                Component::CurDir => None,
                Component::ParentDir => Some(Err(ProjectGlobError::ParentTraversal {
                    pattern: pattern.to_string(),
                })),
                Component::RootDir | Component::Prefix(_) => {
                    Some(Err(ProjectGlobError::NotRelative {
                        pattern: pattern.to_string(),
                    }))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let normalized =
            Pattern::new(&components.join("/")).map_err(|source| ProjectGlobError::Invalid {
                pattern: pattern.to_string(),
                source,
            })?;
        Ok(Self {
            pattern: normalized,
        })
    }

    /// The pattern to pass to [`glob::glob`] to match beneath `project_root`.
    ///
    /// The project root is escaped, so that a `*`, `?`, `[`, or `]` in its
    /// path is matched literally.
    ///
    /// # Errors
    ///
    /// Returns [`FindFilesError::NonUtf8Root`] if the project root is not
    /// valid UTF-8.
    pub fn rooted_pattern(&self, project_root: &Path) -> Result<String, FindFilesError> {
        let root = project_root
            .to_str()
            .ok_or_else(|| FindFilesError::NonUtf8Root {
                path: project_root.to_path_buf(),
            })?;
        Ok(format!(
            "{}/{}",
            Pattern::escape(root),
            self.pattern.as_str()
        ))
    }

    /// Find the files under `project_root` that match the pattern, as paths
    /// relative to `project_root`.
    ///
    /// Symbolic links are followed. Only directories that can contain a
    /// match are read.
    ///
    /// # Errors
    ///
    /// Returns an error if a directory that can contain a match cannot be
    /// read.
    pub fn find_files(&self, project_root: &Path) -> Result<Vec<PathBuf>, FindFilesError> {
        let paths = glob::glob(&self.rooted_pattern(project_root)?).map_err(|source| {
            FindFilesError::InvalidRoot {
                path: project_root.to_path_buf(),
                source,
            }
        })?;
        paths
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| {
                let relative_path = path.strip_prefix(project_root).map(Path::to_path_buf);
                relative_path.map_err(|_| FindFilesError::OutsideRoot { path })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_absolute_pattern() {
        let pattern = if cfg!(windows) { r"C:\*.txt" } else { "/etc/*" };
        assert!(matches!(
            ProjectGlob::parse(pattern),
            Err(ProjectGlobError::NotRelative { .. })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn parse_rejects_drive_relative_patterns() {
        for pattern in [r"\secrets\*", "C:secrets/*"] {
            assert!(matches!(
                ProjectGlob::parse(pattern),
                Err(ProjectGlobError::NotRelative { .. })
            ));
        }
    }

    #[test]
    fn parse_rejects_parent_traversal() {
        for pattern in ["../*", "a/../../b/*", "**/../*"] {
            assert!(
                matches!(
                    ProjectGlob::parse(pattern),
                    Err(ProjectGlobError::ParentTraversal { .. })
                ),
                "{pattern} should be rejected"
            );
        }
    }

    #[test]
    fn parse_rejects_invalid_glob() {
        assert!(matches!(
            ProjectGlob::parse("a**b"),
            Err(ProjectGlobError::Invalid { .. })
        ));
    }

    #[test]
    fn parse_normalizes_the_pattern() {
        let cases = [
            ("*.txt", "*.txt"),
            ("./results/*.csv", "results/*.csv"),
            ("results/./**/*.csv", "results/**/*.csv"),
            ("results//*.csv", "results/*.csv"),
        ];
        for (pattern, expected) in cases {
            assert_eq!(
                ProjectGlob::parse(pattern).unwrap().pattern.as_str(),
                expected
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn backslash_is_literal_on_unix() {
        let glob = ProjectGlob::parse(r"report\2026.txt").unwrap();
        assert_eq!(glob.pattern.as_str(), r"report\2026.txt");
    }

    #[cfg(windows)]
    #[test]
    fn backslash_is_separator_on_windows() {
        let glob = ProjectGlob::parse(r"results\*.csv").unwrap();
        assert_eq!(glob.pattern.as_str(), "results/*.csv");
    }

    #[test]
    fn rooted_pattern_escapes_the_project_root() {
        let glob = ProjectGlob::parse("*.txt").unwrap();
        assert_eq!(
            glob.rooted_pattern(Path::new("/home/me/run[1]*?")).unwrap(),
            "/home/me/run[[]1[]][*][?]/*.txt"
        );
    }
}
