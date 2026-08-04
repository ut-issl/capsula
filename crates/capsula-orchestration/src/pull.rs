use anyhow::{Context, Result};
use capsula_api_types::{RunDetailResponse, RunRecord};
use capsula_core::run::{run_dir_relative_path, setup_vault};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use tracing::{debug, info, warn};
use ulid::Ulid;

/// Pull a single run from the server and restore it under `target_vault_dir`.
///
/// The run directory is reconstructed at `{YYYY-MM-DD}/{HHMMSS}-{name}`
/// (derived from the run's ULID and name, like locally created runs).
/// Captured files are verified against the server-recorded SHA-256 hashes;
/// the `_capsula` metadata files are reconstructed best-effort from the
/// server's structured data and a `_capsula/pulled.json` marker records the
/// origin. Everything is downloaded into a temporary directory and renamed
/// into place, so an interrupted pull never leaves a half-restored run.
///
/// Returns the path of the restored run directory.
pub fn pull_single_run(
    run_id: &str,
    expected_vault: &str,
    target_vault_dir: &Path,
    client: &capsula_client::CapsulaClient,
    force: bool,
) -> Result<PathBuf> {
    let detail = client.get_run(run_id)?;
    let run = match detail.status.as_str() {
        "ok" => detail
            .run
            .as_ref()
            .context("Server response is missing the run record")?,
        "not_found" => anyhow::bail!("Run '{run_id}' not found on server"),
        other => anyhow::bail!(
            "Server returned status '{other}': {}",
            detail.error.as_deref().unwrap_or("unknown error")
        ),
    };

    if run.vault != expected_vault {
        anyhow::bail!(
            "Run '{run_id}' belongs to vault '{}', not '{expected_vault}'. \
             Pass --vault {} to pull it.",
            run.vault,
            run.vault
        );
    }

    let ulid = Ulid::from_string(&run.id)
        .with_context(|| format!("Server returned a non-ULID run id: '{}'", run.id))?;
    let target_dir = target_vault_dir.join(run_dir_relative_path(&ulid, &run.name));

    if target_dir.exists() && !force {
        anyhow::bail!(
            "Run directory already exists: {}. Use --force to replace it.",
            target_dir.display()
        );
    }

    setup_vault(target_vault_dir)?;
    let date_dir = target_dir
        .parent()
        .context("Run directory has no parent directory")?;
    std::fs::create_dir_all(date_dir)
        .with_context(|| format!("Failed to create {}", date_dir.display()))?;

    // Download into a temporary sibling directory (same filesystem) first,
    // then rename into place.
    let temp = tempfile::Builder::new()
        .prefix(".pull-tmp-")
        .tempdir_in(date_dir)
        .context("Failed to create temporary download directory")?;

    download_captured_files(client, &detail, &run.id, temp.path())?;
    write_capsula_dir(&detail, run, &target_dir, temp.path(), client.base_url())?;

    if target_dir.exists() {
        // Only reachable with --force (checked above).
        std::fs::remove_dir_all(&target_dir)
            .with_context(|| format!("Failed to remove {}", target_dir.display()))?;
    }
    std::fs::rename(temp.path(), &target_dir).with_context(|| {
        format!(
            "Failed to move pulled run into place at {}",
            target_dir.display()
        )
    })?;
    // `temp`'s cleanup-on-drop finds the directory already moved; harmless.

    info!("Pulled run '{}' into {}", run.name, target_dir.display());
    Ok(target_dir)
}

/// Download all captured files listed in the run detail into `dest`,
/// verifying each against its server-recorded SHA-256 hash.
fn download_captured_files(
    client: &capsula_client::CapsulaClient,
    detail: &RunDetailResponse,
    run_id: &str,
    dest: &Path,
) -> Result<()> {
    for file in &detail.files {
        let relative_path = sanitize_relative_path(&file.path)?;
        let bytes = client
            .download_file(run_id, &file.path)
            .with_context(|| format!("Failed to download '{}'", file.path))?;

        if let Some(expected) = &file.hash {
            let actual = hex_encode(Sha256::digest(&bytes).as_ref());
            if !actual.eq_ignore_ascii_case(expected) {
                anyhow::bail!(
                    "Hash mismatch for '{}': server recorded {expected}, downloaded {actual}",
                    file.path
                );
            }
        } else {
            warn!(
                "No hash recorded for '{}'; restoring without verification",
                file.path
            );
        }

        let dest_path = dest.join(&relative_path);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        std::fs::write(&dest_path, &bytes)
            .with_context(|| format!("Failed to write {}", dest_path.display()))?;
        debug!("Restored {} ({} bytes)", file.path, bytes.len());
    }
    Ok(())
}

/// Minimal containment check for server-provided file paths: reject
/// anything that is absolute or steps outside the run directory. (Broader
/// hardening against malicious server responses is tracked separately.)
fn sanitize_relative_path(path: &str) -> Result<PathBuf> {
    let p = Path::new(path);
    if p.as_os_str().is_empty()
        || p.is_absolute()
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        anyhow::bail!("Refusing to restore file with unsafe path: '{path}'");
    }
    Ok(p.to_path_buf())
}

/// Reconstruct the `_capsula` directory from the server's structured data.
///
/// The result is a best-effort restoration: the server does not store the
/// original JSON bytes, so formatting and precision (e.g. sub-millisecond
/// duration) differ from a locally produced run.
fn write_capsula_dir(
    detail: &RunDetailResponse,
    run: &RunRecord,
    final_run_dir: &Path,
    dest: &Path,
    server_url: &str,
) -> Result<()> {
    let capsula_dir = dest.join("_capsula");
    std::fs::create_dir_all(&capsula_dir)
        .with_context(|| format!("Failed to create {}", capsula_dir.display()))?;

    // metadata.json — same fields as a locally created run; `run_dir`
    // points at the restored location.
    let metadata = serde_json::json!({
        "id": run.id,
        "name": run.name,
        "command": parse_command(&run.command),
        "timestamp": run.timestamp,
        "run_dir": final_run_dir.to_string_lossy(),
        "project_root": run.project_root,
    });
    write_pretty_json(&capsula_dir.join("metadata.json"), &metadata)?;

    if !detail.pre_run_hooks.is_empty() {
        write_pretty_json(
            &capsula_dir.join("pre-run.json"),
            &serde_json::Value::Array(detail.pre_run_hooks.clone()),
        )?;
    }
    if !detail.post_run_hooks.is_empty() {
        write_pretty_json(
            &capsula_dir.join("post-run.json"),
            &serde_json::Value::Array(detail.post_run_hooks.clone()),
        )?;
    }

    // command.json — only finalized runs have one; exit_code is the marker.
    // The server stores the duration in milliseconds, so the restored
    // duration loses sub-millisecond precision.
    if let Some(exit_code) = run.exit_code {
        let duration_ms = u64::try_from(run.duration_ms.unwrap_or(0)).unwrap_or(0);
        let command_output = serde_json::json!({
            "exit_code": exit_code,
            "stdout": run.stdout.clone().unwrap_or_default(),
            "stderr": run.stderr.clone().unwrap_or_default(),
            "duration": {
                "secs": duration_ms / 1000,
                "nanos": (duration_ms % 1000) * 1_000_000,
            },
        });
        write_pretty_json(&capsula_dir.join("command.json"), &command_output)?;
    }

    // Marker distinguishing pulled runs from locally produced ones.
    let pulled = serde_json::json!({
        "server_url": server_url,
        "pulled_at": chrono::Utc::now().to_rfc3339(),
    });
    write_pretty_json(&capsula_dir.join("pulled.json"), &pulled)?;

    Ok(())
}

/// The server stores the command as a JSON array string; fall back to
/// shell-splitting for records that predate that format.
fn parse_command(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .ok()
        .or_else(|| shlex::split(raw))
        .unwrap_or_else(|| vec![raw.to_string()])
}

fn write_pretty_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "Tests use unwrap/expect for brevity"
    )]

    use super::*;

    #[test]
    fn parse_command_reads_json_array_string() {
        assert_eq!(
            parse_command(r#"["echo","hello world"]"#),
            vec!["echo".to_string(), "hello world".to_string()]
        );
    }

    #[test]
    fn parse_command_falls_back_to_shell_splitting() {
        assert_eq!(
            parse_command("echo 'hello world'"),
            vec!["echo".to_string(), "hello world".to_string()]
        );
    }

    #[test]
    fn sanitize_rejects_escaping_paths() {
        assert!(sanitize_relative_path("../evil.txt").is_err());
        assert!(sanitize_relative_path("/etc/passwd").is_err());
        assert!(sanitize_relative_path("a/../../evil.txt").is_err());
        assert!(sanitize_relative_path("").is_err());
    }

    #[test]
    fn sanitize_accepts_nested_relative_paths() {
        assert_eq!(
            sanitize_relative_path("output/result.txt").unwrap(),
            PathBuf::from("output/result.txt")
        );
    }
}
