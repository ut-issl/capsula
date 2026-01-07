use std::process::Command;

fn main() {
    // Get git commit hash
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output();

    let git_hash = match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        _ => None,
    };

    // Set GIT_HASH to actual hash or empty string if not available
    let git_hash_value = git_hash.as_deref().unwrap_or("");
    println!("cargo:rustc-env=GIT_HASH={git_hash_value}");

    // Rerun if HEAD changes (new commit)
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
