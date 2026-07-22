//! Small pure-helper utilities shared across Capsula crates.

use crate::project_path::ResolvedProjectPath;
use std::path::{Path, PathBuf};

/// Encode bytes as a lowercase hexadecimal string.
#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut output, b| {
            use std::fmt::Write;
            let _ = write!(output, "{b:02x}");
            output
        })
}

/// Resolve a path that may be absolute or relative to `project_root`.
///
/// Both absolute and relative paths are canonicalized, and the result must stay
/// within the canonical project root. Prefer using [`ResolvedProjectPath`] when
/// the containment invariant matters beyond a single helper call.
pub fn resolve_relative(path: &Path, project_root: &Path) -> std::io::Result<PathBuf> {
    ResolvedProjectPath::resolve_existing(path, project_root)
        .map(ResolvedProjectPath::into_path_buf)
        .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::{hex_encode, resolve_relative};
    use std::path::Path;

    #[test]
    fn hex_encode_empty() {
        assert_eq!(hex_encode(&[]), "");
    }

    #[test]
    fn hex_encode_single_bytes() {
        assert_eq!(hex_encode(&[0x00]), "00");
        assert_eq!(hex_encode(&[0xff]), "ff");
        assert_eq!(hex_encode(&[0xab, 0xcd, 0xef]), "abcdef");
    }

    #[test]
    fn resolve_relative_canonicalizes_absolute_path_inside_project() {
        let cwd = std::env::current_dir().unwrap();
        let resolved = resolve_relative(&cwd, &cwd).unwrap();
        assert_eq!(resolved, cwd.canonicalize().unwrap());
    }

    #[test]
    fn resolve_relative_joins_relative_and_canonicalizes() {
        // project_root = cwd so canonicalize can resolve it.
        let cwd = std::env::current_dir().unwrap();
        let resolved = resolve_relative(Path::new("."), &cwd).unwrap();
        // canonicalize resolves symlinks; on macOS this is /private/var/... etc.
        assert_eq!(resolved, cwd.canonicalize().unwrap());
    }

    #[test]
    fn resolve_relative_rejects_absolute_path_outside_project() {
        let cwd = std::env::current_dir().unwrap();
        let outside = cwd.parent().unwrap_or(&cwd);

        if outside != cwd {
            let result = resolve_relative(outside, &cwd);
            assert!(result.is_err());
        }
    }
}
