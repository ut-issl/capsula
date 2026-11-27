//! Helpers for user-supplied file paths and download headers.
//!
//! Used by the upload and download endpoints: captured-file paths are
//! validated with [`sanitize_relative_path`] before they are stored, and
//! downloads name the file with [`content_disposition_inline`].

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use std::path::{Component, Path};

// RFC 5987 `attr-char` set: ALPHA / DIGIT plus `!#$&+-.^_`|~`. Everything else
// must be percent-encoded when used in a `filename*=UTF-8''...` parameter.
const RFC5987_ATTR_CHAR: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'!')
    .remove(b'#')
    .remove(b'$')
    .remove(b'&')
    .remove(b'+')
    .remove(b'-')
    .remove(b'.')
    .remove(b'^')
    .remove(b'_')
    .remove(b'`')
    .remove(b'|')
    .remove(b'~');

/// Build an RFC 6266 / RFC 5987 `Content-Disposition` header value for `filename`.
///
/// Emits both a safe ASCII `filename=` fallback and a percent-encoded
/// `filename*=UTF-8''...` parameter so arbitrary bytes (including quotes, CR/LF,
/// and control characters) in the filename cannot inject additional headers.
#[must_use]
pub fn content_disposition_inline(filename: &str) -> String {
    let ascii_fallback: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let ascii_fallback = if ascii_fallback.is_empty() {
        "download".to_owned()
    } else {
        ascii_fallback
    };
    let encoded = utf8_percent_encode(filename, RFC5987_ATTR_CHAR);
    format!("inline; filename=\"{ascii_fallback}\"; filename*=UTF-8''{encoded}")
}

/// Validate a user-supplied relative path for the `captured_files.path` column.
///
/// Rejects absolute paths, path-traversal segments, Windows drive prefixes, NUL
/// bytes, and empty values so downstream callers (download URLs, local
/// reconstruction) cannot be tricked into escaping the run's virtual root.
pub fn sanitize_relative_path(candidate: &str) -> Result<String, &'static str> {
    if candidate.is_empty() {
        return Err("path is empty");
    }
    if candidate.contains('\0') {
        return Err("path contains NUL byte");
    }

    // Normalize backslashes to forward slashes so Windows-style separators are
    // parsed consistently on all platforms. `Component::Prefix` is only emitted
    // when `std::path` parses on Windows, so drive letters are detected
    // explicitly below rather than relying on the path parser alone.
    let normalized = candidate.replace('\\', "/");
    let first_segment = normalized.split('/').next().unwrap_or("");
    let mut first_chars = first_segment.chars();
    if first_chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && first_chars.next() == Some(':')
    {
        return Err("path must be relative");
    }

    let mut out = String::with_capacity(normalized.len());
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(seg) => {
                let seg = seg.to_str().ok_or("path contains invalid UTF-8")?;
                if !out.is_empty() {
                    out.push('/');
                }
                out.push_str(seg);
            }
            Component::CurDir => {}
            Component::ParentDir => return Err("path contains '..' segment"),
            Component::RootDir | Component::Prefix(_) => return Err("path must be relative"),
        }
    }

    if out.is_empty() {
        return Err("path is empty");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{content_disposition_inline, sanitize_relative_path};

    #[test]
    fn sanitize_rejects_absolute_paths() {
        assert!(sanitize_relative_path("/etc/passwd").is_err());
        assert!(sanitize_relative_path("\\windows\\system32").is_err());
    }

    #[test]
    fn sanitize_rejects_parent_traversal() {
        assert!(sanitize_relative_path("../etc/passwd").is_err());
        assert!(sanitize_relative_path("a/../b").is_err());
        assert!(sanitize_relative_path("a\\..\\b").is_err());
    }

    #[test]
    fn sanitize_rejects_drive_prefix_and_nul() {
        assert!(sanitize_relative_path("C:\\Users").is_err());
        assert!(sanitize_relative_path("d:/foo").is_err());
        assert!(sanitize_relative_path("foo\0bar").is_err());
        assert!(sanitize_relative_path("").is_err());
    }

    #[test]
    fn sanitize_accepts_normal_paths() {
        assert_eq!(sanitize_relative_path("foo.txt").unwrap(), "foo.txt");
        assert_eq!(
            sanitize_relative_path("dir/sub/file.log").unwrap(),
            "dir/sub/file.log"
        );
        // Backslashes are normalized to forward slashes.
        assert_eq!(
            sanitize_relative_path("dir\\sub\\file.log").unwrap(),
            "dir/sub/file.log"
        );
    }

    #[test]
    fn sanitize_normalizes_redundant_segments() {
        // `Path::components()` collapses empty segments and drops `.` segments.
        assert_eq!(sanitize_relative_path("a//b").unwrap(), "a/b");
        assert_eq!(sanitize_relative_path("./a/b").unwrap(), "a/b");
        assert_eq!(sanitize_relative_path("a/./b").unwrap(), "a/b");
        assert_eq!(sanitize_relative_path("a/b/").unwrap(), "a/b");
    }

    #[test]
    fn content_disposition_strips_dangerous_chars_in_fallback() {
        let header = content_disposition_inline("a\"b\r\nc.txt");
        assert!(header.starts_with("inline; filename=\"a_b__c.txt\";"));
        // Must not contain a bare CR or LF in the header.
        assert!(!header.contains('\r'));
        assert!(!header.contains('\n'));
    }

    #[test]
    fn content_disposition_encodes_unicode_in_filename_star() {
        let header = content_disposition_inline("日本語.txt");
        assert!(header.contains("filename*=UTF-8''"));
        // UTF-8 for '日' starts with 0xE6 0x97 0xA5.
        assert!(header.contains("%E6%97%A5"));
    }

    #[test]
    fn content_disposition_percent_encodes_attr_char_boundary() {
        // Unreserved ASCII round-trips; spaces and quotes are percent-encoded.
        let header = content_disposition_inline("abc.txt");
        assert!(header.contains("filename*=UTF-8''abc.txt"));
        let header = content_disposition_inline("a b\"c");
        assert!(header.contains("filename*=UTF-8''a%20b%22c"));
    }
}
