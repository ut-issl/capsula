//! Tests for the build script's `rerun-if-changed` path selection.
#![cfg(test)]

#[path = "../build/rerun_paths.rs"]
mod rerun_paths;

use rerun_paths::{GitPaths, rerun_paths};
use std::path::{Path, PathBuf};

const HEAD: &str = "/repo/.git/worktrees/wt/HEAD";
const PACKED_REFS: &str = "/repo/.git/packed-refs";

fn select(current_ref: Option<(&str, &str)>, existing: &[&str]) -> Vec<PathBuf> {
    let paths = GitPaths {
        head: PathBuf::from(HEAD),
        packed_refs: PathBuf::from(PACKED_REFS),
        current_ref: current_ref.map(|(name, path)| (name.to_string(), PathBuf::from(path))),
    };
    rerun_paths(&paths, |path: &Path| {
        existing.iter().any(|existing| Path::new(existing) == path)
    })
}

fn paths(expected: &[&str]) -> Vec<PathBuf> {
    expected.iter().map(PathBuf::from).collect()
}

#[test]
fn branch_with_loose_ref_and_packed_refs() {
    let selected = select(
        Some(("refs/heads/main", "/repo/.git/refs/heads/main")),
        &[HEAD, PACKED_REFS, "/repo/.git/refs/heads/main"],
    );
    assert_eq!(
        selected,
        paths(&[HEAD, PACKED_REFS, "/repo/.git/refs/heads/main"])
    );
}

#[test]
fn missing_packed_refs_is_not_watched() {
    let selected = select(
        Some(("refs/heads/main", "/repo/.git/refs/heads/main")),
        &[HEAD, "/repo/.git/refs/heads/main"],
    );
    assert_eq!(selected, paths(&[HEAD, "/repo/.git/refs/heads/main"]));
}

#[test]
fn packed_only_ref_watches_parent_directory() {
    let selected = select(
        Some(("refs/heads/main", "/repo/.git/refs/heads/main")),
        &[HEAD, PACKED_REFS, "/repo/.git/refs/heads"],
    );
    assert_eq!(
        selected,
        paths(&[HEAD, PACKED_REFS, "/repo/.git/refs/heads"])
    );
}

#[test]
fn packed_only_nested_ref_watches_nearest_existing_ancestor() {
    let selected = select(
        Some(("refs/heads/feature/x", "/repo/.git/refs/heads/feature/x")),
        &[HEAD, PACKED_REFS, "/repo/.git/refs/heads"],
    );
    assert_eq!(
        selected,
        paths(&[HEAD, PACKED_REFS, "/repo/.git/refs/heads"])
    );
}

#[test]
fn ancestor_search_stops_at_refs_directory() {
    let selected = select(
        Some(("refs/heads/main", "/repo/.git/refs/heads/main")),
        &[HEAD, PACKED_REFS, "/repo/.git"],
    );
    assert_eq!(selected, paths(&[HEAD, PACKED_REFS]));
}

#[test]
fn detached_head_watches_head_and_packed_refs() {
    let selected = select(None, &[HEAD, PACKED_REFS]);
    assert_eq!(selected, paths(&[HEAD, PACKED_REFS]));
}

#[test]
fn relative_paths_are_kept_as_given() {
    let selected = select(
        Some(("refs/heads/main", "../../.git/refs/heads/main")),
        &[HEAD, "../../.git/refs/heads/main"],
    );
    assert_eq!(selected, paths(&[HEAD, "../../.git/refs/heads/main"]));
}
