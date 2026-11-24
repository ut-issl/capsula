//! Opening matched source files, and removing them in move mode only if
//! nothing was written to them after they were archived.

use crate::error::FileHookError;
use same_file::Handle;
use std::fs::File;
use std::path::Path;
use std::time::SystemTime;

/// The size and modification time of a file, used to tell whether it changed
/// after it was copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileState {
    len: u64,
    modified: Option<SystemTime>,
}

impl FileState {
    /// The state of a file as described by `metadata`.
    #[must_use]
    pub fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }

    /// The state of an archived copy of a file that was in state `self` when
    /// it was opened and of which `copied_len` bytes were copied.
    #[must_use]
    pub const fn archived(self, copied_len: u64) -> Self {
        Self {
            len: copied_len,
            ..self
        }
    }
}

/// Open a matched source for reading. The opened file must be a regular
/// file.
///
/// # Errors
///
/// Returns [`FileHookError::OpenSource`] if the path cannot be opened, and
/// [`FileHookError::SourceNotRegularFile`] if the opened file is not a
/// regular file.
pub fn open_source(path: &Path) -> Result<File, FileHookError> {
    let file = File::open(path).map_err(|source| FileHookError::OpenSource {
        path: path.to_path_buf(),
        source,
    })?;
    if file.metadata()?.is_file() {
        Ok(file)
    } else {
        Err(FileHookError::SourceNotRegularFile {
            path: path.to_path_buf(),
        })
    }
}

/// Remove a source if `path` still names `opened`, the file that was
/// archived, and that file is still in the `archived` state, so that no byte
/// written to it after it was opened is lost.
///
/// `opened` must stay open until this returns, so that its identity cannot
/// be reused by another file.
///
/// # Errors
///
/// Returns [`FileHookError::SourceChanged`] or
/// [`FileHookError::SourceReplaced`] without removing anything if the file
/// changed or the path now names a different file, and
/// [`FileHookError::RemoveMovedSource`] if it cannot be checked or removed.
pub fn remove_if_unchanged(
    path: &Path,
    opened: &File,
    archived: FileState,
) -> Result<(), FileHookError> {
    let remove_error = |source| FileHookError::RemoveMovedSource {
        path: path.to_path_buf(),
        source,
    };
    let replaced = || FileHookError::SourceReplaced {
        path: path.to_path_buf(),
    };

    if FileState::of(&opened.metadata().map_err(remove_error)?) != archived {
        return Err(FileHookError::SourceChanged {
            path: path.to_path_buf(),
        });
    }

    let current = match open_source(path) {
        Ok(file) => file,
        Err(FileHookError::SourceNotRegularFile { .. }) => return Err(replaced()),
        Err(FileHookError::OpenSource { source, .. }) => return Err(remove_error(source)),
        Err(error) => return Err(error),
    };
    let unchanged = Handle::from_file(current).map_err(remove_error)?
        == Handle::from_file(opened.try_clone().map_err(remove_error)?).map_err(remove_error)?;
    if !unchanged {
        return Err(replaced());
    }
    std::fs::remove_file(path).map_err(remove_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn state_of(file: &File) -> FileState {
        FileState::of(&file.metadata().unwrap())
    }

    #[test]
    fn open_source_opens_regular_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.txt");
        fs::write(&path, b"a").unwrap();

        assert!(open_source(&path).is_ok());
    }

    #[test]
    fn open_source_rejects_directory() {
        let temp = tempfile::tempdir().unwrap();

        assert!(matches!(
            open_source(temp.path()),
            Err(FileHookError::SourceNotRegularFile { .. })
        ));
    }

    #[test]
    fn remove_if_unchanged_removes_the_opened_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.txt");
        fs::write(&path, b"a").unwrap();
        let opened = open_source(&path).unwrap();

        remove_if_unchanged(&path, &opened, state_of(&opened)).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn remove_if_unchanged_keeps_a_file_that_grew() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.txt");
        fs::write(&path, b"old").unwrap();
        let opened = open_source(&path).unwrap();
        let archived = state_of(&opened);
        let mut writer = fs::OpenOptions::new().append(true).open(&path).unwrap();
        std::io::Write::write_all(&mut writer, b" and new").unwrap();

        let result = remove_if_unchanged(&path, &opened, archived);

        assert!(matches!(result, Err(FileHookError::SourceChanged { .. })));
        assert_eq!(fs::read(&path).unwrap(), b"old and new");
    }

    #[test]
    fn remove_if_unchanged_keeps_a_file_copied_only_in_part() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.txt");
        fs::write(&path, b"old").unwrap();
        let opened = open_source(&path).unwrap();
        let archived = state_of(&opened).archived(2);

        let result = remove_if_unchanged(&path, &opened, archived);

        assert!(matches!(result, Err(FileHookError::SourceChanged { .. })));
        assert!(path.exists());
    }

    #[test]
    fn remove_if_unchanged_keeps_a_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.txt");
        let replacement = temp.path().join("a.txt.tmp");
        fs::write(&path, b"old").unwrap();
        let opened = open_source(&path).unwrap();
        fs::write(&replacement, b"new").unwrap();
        fs::rename(&replacement, &path).unwrap();

        let result = remove_if_unchanged(&path, &opened, state_of(&opened));

        assert!(matches!(result, Err(FileHookError::SourceReplaced { .. })));
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }
}
