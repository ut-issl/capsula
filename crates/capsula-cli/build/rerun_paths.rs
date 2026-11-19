//! Pure decision logic for the build script's `cargo:rerun-if-changed` paths.
//!
//! Shared by `build.rs` and the `build_rerun_paths` integration test through
//! `#[path]`, because a build script cannot carry unit tests of its own.

use std::path::{Path, PathBuf};

/// The locations git reports for the files that determine the current commit.
pub struct GitPaths {
    /// `git rev-parse --git-path HEAD`.
    pub head: PathBuf,
    /// `git rev-parse --git-path packed-refs`.
    pub packed_refs: PathBuf,
    /// The ref `HEAD` points to (`git symbolic-ref -q HEAD`) and its loose ref
    /// file (`git rev-parse --git-path <ref>`); `None` when `HEAD` is detached.
    pub current_ref: Option<(String, PathBuf)>,
}

/// Returns the paths whose change means the current commit may have changed.
///
/// Cargo re-runs a build script on every build when a watched path does not
/// exist, so only existing paths are returned:
///
/// - `HEAD` always; on a detached `HEAD` it holds the commit itself.
/// - `packed-refs` when it exists. While it is absent, the current ref can
///   only be a loose ref file, which is watched instead.
/// - The loose ref file of the current branch when it exists. When the ref is
///   only packed, a commit creates the loose file, so the nearest existing
///   ancestor directory inside the `refs/` directory is watched instead;
///   Cargo scans a watched directory recursively.
pub fn rerun_paths(paths: &GitPaths, exists: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let packed_refs = Some(paths.packed_refs.clone()).filter(|path| exists(path));
    let current_ref = paths
        .current_ref
        .as_ref()
        .and_then(|(name, path)| loose_ref_watch_path(name, path, &exists));

    std::iter::once(paths.head.clone())
        .chain(packed_refs)
        .chain(current_ref)
        .collect()
}

/// The loose ref file if it exists, otherwise its nearest existing ancestor
/// that is still inside the `refs/` directory the ref name starts from.
fn loose_ref_watch_path(
    ref_name: &str,
    ref_path: &Path,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    // `refs/heads/main` lives at `<common dir>/refs/heads/main`: the file and
    // its ancestors up to `<common dir>/refs` are one per name component.
    let depth = Path::new(ref_name).components().count();
    ref_path
        .ancestors()
        .take(depth)
        .find(|path| exists(path))
        .map(Path::to_path_buf)
}
