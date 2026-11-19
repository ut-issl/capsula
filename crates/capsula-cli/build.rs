use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "build/rerun_paths.rs"]
mod rerun_paths;

use rerun_paths::{GitPaths, rerun_paths};

/// Runs `git` with `args` and returns its trimmed stdout, or `None` when git is
/// unavailable, the build is outside a repository, or the command fails.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Asks git where the files that determine the current commit live. Paths are
/// relative to the package root (the build script's working directory) or
/// absolute, both of which Cargo accepts.
fn git_paths() -> Option<GitPaths> {
    let git_path = |path: &str| git(&["rev-parse", "--git-path", path]).map(PathBuf::from);
    let current_ref = git(&["symbolic-ref", "-q", "HEAD"])
        .and_then(|name| git_path(&name).map(|path| (name, path)));
    Some(GitPaths {
        head: git_path("HEAD")?,
        packed_refs: git_path("packed-refs")?,
        current_ref,
    })
}

fn main() {
    // Set GIT_HASH to the commit hash or an empty string if not available
    let git_hash = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_default();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    // Rerun when the current commit changes. Outside a git repository nothing
    // is emitted, so Cargo falls back to rerunning on any package change.
    if let Some(paths) = git_paths() {
        for path in rerun_paths(&paths, Path::exists) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
