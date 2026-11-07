//! Writing captured files into the hook's artifact directory, under their
//! paths relative to the project root, without overwriting anything.

use crate::error::FileHookError;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tracing::warn;

/// The hook's artifact directory.
#[derive(Debug)]
pub struct ArtifactDir {
    path: PathBuf,
}

impl ArtifactDir {
    /// The artifact directory at `path`. Nothing is created or checked until
    /// an artifact is written.
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// The path of an artifact given relative to the artifact directory.
    #[must_use]
    pub fn path_of(&self, relative_path: &Path) -> PathBuf {
        self.path.join(relative_path)
    }

    /// Whether anything, including a symbolic link, exists at
    /// `relative_path` in the artifact directory.
    ///
    /// # Errors
    ///
    /// Returns [`FileHookError::ArtifactPath`] if the path cannot be checked.
    pub fn exists(&self, relative_path: &Path) -> Result<bool, FileHookError> {
        let path = self.path_of(relative_path);
        match path.symlink_metadata() {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(source) => Err(FileHookError::ArtifactPath { path, source }),
        }
    }

    /// Create a new file at `relative_path` in the artifact directory,
    /// creating its parent directories.
    ///
    /// # Errors
    ///
    /// Returns [`FileHookError::ArtifactExists`] if something already exists
    /// at the path, and [`FileHookError::ArtifactPath`] if the file or its
    /// parent directories cannot be created.
    pub fn create_new(&self, relative_path: &Path) -> Result<File, FileHookError> {
        let path = self.path_of(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FileHookError::ArtifactPath {
                path: path.clone(),
                source,
            })?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| match source.kind() {
                ErrorKind::AlreadyExists => FileHookError::ArtifactExists { path: path.clone() },
                _ => FileHookError::ArtifactPath {
                    path: path.clone(),
                    source,
                },
            })
    }

    /// Remove a partially written artifact, logging a failure.
    pub fn remove_partial(&self, relative_path: &Path) {
        let path = self.path_of(relative_path);
        if let Err(error) = std::fs::remove_file(&path) {
            warn!(
                "FileHook: Failed to remove partial artifact {}: {error}",
                path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn create_new_creates_parent_directories() {
        let temp = tempfile::tempdir().unwrap();
        let artifacts = ArtifactDir::new(temp.path());

        artifacts.create_new(Path::new("a/b/c.txt")).unwrap();

        assert!(temp.path().join("a/b/c.txt").is_file());
    }

    #[test]
    fn create_new_refuses_an_existing_file() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("c.txt"), b"existing").unwrap();
        let artifacts = ArtifactDir::new(temp.path());

        assert!(matches!(
            artifacts.create_new(Path::new("c.txt")),
            Err(FileHookError::ArtifactExists { .. })
        ));
        assert_eq!(fs::read(temp.path().join("c.txt")).unwrap(), b"existing");
    }

    #[cfg(unix)]
    #[test]
    fn exists_sees_a_broken_symlink() {
        let temp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("missing", temp.path().join("link")).unwrap();
        let artifacts = ArtifactDir::new(temp.path());

        assert!(artifacts.exists(Path::new("link")).unwrap());
        assert!(!artifacts.exists(Path::new("other")).unwrap());
    }
}
