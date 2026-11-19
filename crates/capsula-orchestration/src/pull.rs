use crate::resolve::VaultPathSource;
use anyhow::{Context, Result};
use capsula_api_types::{RunDetailFile, RunDetailResponse, RunRecord};
use capsula_config::default_vault_path;
use capsula_core::run::{run_dir_relative_path, setup_vault};
use std::path::{Component, Path, PathBuf};
use tracing::{debug, info, warn};
use ulid::Ulid;

/// Pull a single run from the server and restore it under `target_vault_dir`.
///
/// The run directory is reconstructed at `{YYYY-MM-DD}/{HHMMSS}-{name}`
/// (derived from the run's ULID and name, like locally created runs).
/// Captured files are verified against the server-recorded sizes and
/// SHA-256 hashes; the `_capsula` metadata files are reconstructed
/// best-effort from the server's structured data and a
/// `_capsula/pulled.json` marker records the origin. Everything is
/// downloaded into a temporary directory directly under the vault root
/// (outside the `vault/*/*` depth that the vault scanners walk) and
/// renamed into place, so an interrupted pull never leaves a
/// half-restored or phantom run.
///
/// A pulled run is a lossy reconstruction of the original, so `--force`
/// only replaces run directories that carry the `pulled.json` marker;
/// locally produced runs are never replaced.
///
/// Returns the path of the restored run directory.
pub fn pull_single_run(
    run_id: &str,
    expected_vault: &str,
    target_vault_dir: &Path,
    client: &capsula_client::CapsulaClient,
    force: bool,
) -> Result<PathBuf> {
    // Fail fast on non-ULID arguments (e.g. a run *name*): the server
    // would answer not_found, which misleadingly reads as "never pushed".
    if Ulid::from_string(run_id).is_err() {
        anyhow::bail!(
            "'{run_id}' is not a run ID (ULID). `capsula pull` requires the run ID; \
             run names cannot be resolved on the server. Find the ID with `capsula show <name>`."
        );
    }

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
    // The name becomes part of the run directory name; a separator or
    // `..` inside it would place the run outside the vault.
    ensure_single_path_component(&run.name, "run name")?;
    let target_dir = target_vault_dir.join(run_dir_relative_path(&ulid, &run.name));

    if target_dir.exists() {
        let is_pulled = is_pulled_run(&target_dir);
        if !force {
            if is_pulled {
                anyhow::bail!(
                    "A previously pulled copy of this run already exists: {}. \
                     Use --force to replace it.",
                    target_dir.display()
                );
            }
            anyhow::bail!(
                "Run directory already exists and appears locally produced \
                 (no _capsula/pulled.json marker): {}. Refusing to replace it: \
                 a pulled run is a lossy reconstruction of the original.",
                target_dir.display()
            );
        }
        if !is_pulled {
            anyhow::bail!(
                "--force only replaces previously pulled runs, but {} has no \
                 _capsula/pulled.json marker and appears locally produced. \
                 A pulled run is a lossy reconstruction of the original; \
                 delete the directory manually if you really intend to replace it.",
                target_dir.display()
            );
        }
    }

    setup_vault(target_vault_dir)?;
    let date_dir = target_dir
        .parent()
        .context("Run directory has no parent directory")?;
    std::fs::create_dir_all(date_dir)
        .with_context(|| format!("Failed to create {}", date_dir.display()))?;

    // Download into a temporary directory directly under the vault root:
    // the same filesystem (so the rename below stays atomic), but one
    // level above the `vault/*/*` depth that list_runs / find_run_dir /
    // `push --all` walk — an interrupted pull leaves at worst an inert
    // `.pull-tmp-*` directory that no scanner mistakes for a run.
    let temp = tempfile::Builder::new()
        .prefix(".pull-tmp-")
        .tempdir_in(target_vault_dir)
        .context("Failed to create temporary download directory")?;

    download_captured_files(client, &detail, &run.id, temp.path())?;
    write_capsula_dir(&detail, run, &target_dir, temp.path(), client.base_url())?;

    // Re-check at replacement time: the directory may have appeared (or
    // changed) while we were downloading, e.g. through a concurrent pull
    // or a local run landing in the same slot.
    if target_dir.exists() {
        if !force || !is_pulled_run(&target_dir) {
            anyhow::bail!(
                "Run directory {} appeared or changed during the download; \
                 refusing to replace it",
                target_dir.display()
            );
        }
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

/// Whether `run_dir` holds a run restored by `pull`.
///
/// A pulled run carries an `_capsula/pulled.json` marker; locally
/// produced runs do not. A pulled run is a lossy reconstruction of the
/// original, so callers use this both to protect local runs from being
/// replaced and to keep pulled runs from being pushed back.
#[must_use]
pub fn is_pulled_run(run_dir: &Path) -> bool {
    run_dir.join("_capsula").join("pulled.json").is_file()
}

/// Decide which vault directory a pulled run is restored into.
///
/// `[vault] path` describes the *configured* vault only, so a run from
/// another vault would land in a location that was never chosen for it
/// (an external directory, for instance). Such a run therefore defaults
/// to that vault's default location under the project root. An explicit
/// `--vault-path` / `CAPSULA_VAULT_PATH` override is a per-invocation
/// choice and always wins.
pub fn resolve_pull_vault_dir(
    vault_name: &str,
    config_vault_name: &str,
    vault_dir: PathBuf,
    vault_path_source: VaultPathSource,
    project_root: &Path,
) -> Result<PathBuf> {
    if vault_name == config_vault_name || vault_path_source == VaultPathSource::Explicit {
        return Ok(vault_dir);
    }

    // The vault name becomes a directory name under the project root;
    // a separator or `..` would place the vault outside it.
    ensure_single_path_component(vault_name, "vault name")?;
    Ok(project_root.join(default_vault_path(vault_name)))
}

/// Assert that `value` is usable as a single, normal path component —
/// no separators, no `..`, not absolute, not empty.
pub fn ensure_single_path_component(value: &str, what: &str) -> Result<()> {
    let mut components = Path::new(value).components();
    let is_single = matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    );
    if !is_single {
        anyhow::bail!("{what} '{value}' is not a single path component");
    }
    Ok(())
}

/// Download all captured files listed in the run detail into `dest`,
/// verifying each against its server-recorded size and SHA-256 hash.
fn download_captured_files(
    client: &capsula_client::CapsulaClient,
    detail: &RunDetailResponse,
    run_id: &str,
    dest: &Path,
) -> Result<()> {
    for file in &detail.files {
        let relative_path = checked_run_relative_path(&file.path)?;
        let dest_path = dest.join(&relative_path);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        let size = download_verified(client, run_id, file, &dest_path)?;
        debug!("Restored {} ({} bytes)", file.path, size);
    }
    Ok(())
}

/// Stream one captured file to `dest_path`, then verify it against the
/// server-recorded size and hash. The file is deleted if the download or
/// the verification fails, so only verified content is ever kept.
///
/// Returns the number of bytes restored.
fn download_verified(
    client: &capsula_client::CapsulaClient,
    run_id: &str,
    file: &RunDetailFile,
    dest_path: &Path,
) -> Result<u64> {
    let result = std::fs::File::create(dest_path)
        .with_context(|| format!("Failed to write {}", dest_path.display()))
        .and_then(|mut out| {
            client
                .download_file_to(run_id, &file.path, &mut out)
                .with_context(|| format!("Failed to download '{}'", file.path))
        })
        .and_then(|(size, digest)| verify_download(file, size, &digest).map(|()| size));

    if result.is_err()
        && let Err(e) = std::fs::remove_file(dest_path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!(
            "Failed to remove unverified download {}: {e}",
            dest_path.display()
        );
    }
    result
}

/// Check a downloaded file's size and SHA-256 digest against the values
/// recorded by the server. A file without a recorded hash is accepted
/// with a warning.
fn verify_download(file: &RunDetailFile, size: u64, digest: &str) -> Result<()> {
    if i64::try_from(size).ok() != Some(file.size) {
        anyhow::bail!(
            "Size mismatch for '{}': server recorded {} bytes, downloaded {}",
            file.path,
            file.size,
            size
        );
    }

    if let Some(expected) = &file.hash {
        if !digest.eq_ignore_ascii_case(expected) {
            anyhow::bail!(
                "Hash mismatch for '{}': server recorded {expected}, downloaded {digest}",
                file.path
            );
        }
    } else {
        warn!(
            "No hash recorded for '{}'; restoring without verification",
            file.path
        );
    }
    Ok(())
}

/// Minimal containment check for server-provided file paths: reject
/// anything that is absolute or steps outside the run directory.
/// (Distinct from the server's upload-side `sanitize_relative_path`,
/// which normalizes instead of rejecting; broader hardening is tracked
/// separately.)
fn checked_run_relative_path(path: &str) -> Result<PathBuf> {
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
        let duration_ms: u64 = match run.duration_ms {
            Some(ms) if ms < 0 => {
                warn!(
                    "Server recorded a negative duration_ms ({ms}) for run '{}'; \
                     restoring duration as 0",
                    run.id
                );
                0
            }
            Some(ms) => u64::try_from(ms).unwrap_or(0),
            None => 0,
        };
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

    // Marker distinguishing pulled runs from locally produced ones; read
    // back by pull's --force guard and by push's re-upload guard.
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

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn run_file(size: i64, hash: Option<&str>) -> RunDetailFile {
        RunDetailFile {
            path: "out/abc.txt".to_string(),
            size,
            hash: hash.map(str::to_string),
        }
    }

    #[test]
    fn verify_download_accepts_matching_size_and_hash_case_insensitively() {
        assert!(verify_download(&run_file(3, Some(ABC_SHA256)), 3, ABC_SHA256).is_ok());
        let upper = ABC_SHA256.to_ascii_uppercase();
        assert!(verify_download(&run_file(3, Some(&upper)), 3, ABC_SHA256).is_ok());
    }

    #[test]
    fn verify_download_accepts_missing_hash() {
        assert!(verify_download(&run_file(3, None), 3, ABC_SHA256).is_ok());
    }

    #[test]
    fn verify_download_rejects_size_mismatch() {
        let err = verify_download(&run_file(4, Some(ABC_SHA256)), 3, ABC_SHA256).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Size mismatch for 'out/abc.txt': server recorded 4 bytes, downloaded 3"
        );
    }

    #[test]
    fn verify_download_rejects_hash_mismatch() {
        let other = "0".repeat(64);
        let err = verify_download(&run_file(3, Some(&other)), 3, ABC_SHA256).unwrap_err();
        assert!(
            err.to_string()
                .starts_with("Hash mismatch for 'out/abc.txt'")
        );
    }

    /// Serve `body` for `out/abc.txt` and stream it to a file via
    /// `download_verified`, returning the result and whether the file
    /// remains on disk.
    fn run_download_verified(file: &RunDetailFile, body: &[u8]) -> (Result<u64>, Option<Vec<u8>>) {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/api/v1/runs/01RUN/files/out/abc.txt"))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
                .mount(&server)
                .await;
            server
        });
        let client = capsula_client::CapsulaClient::new(server.uri());
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("abc.txt");

        let result = download_verified(&client, "01RUN", file, &dest);
        (result, std::fs::read(&dest).ok())
    }

    #[test]
    fn download_verified_keeps_matching_file() {
        let (result, kept) = run_download_verified(&run_file(3, Some(ABC_SHA256)), b"abc");
        assert_eq!(result.unwrap(), 3);
        assert_eq!(kept.as_deref(), Some(&b"abc"[..]));
    }

    #[test]
    fn download_verified_discards_file_on_hash_mismatch() {
        let (result, kept) = run_download_verified(&run_file(3, Some(ABC_SHA256)), b"xyz");
        assert!(result.unwrap_err().to_string().contains("Hash mismatch"));
        assert!(kept.is_none(), "mismatching file must be deleted");
    }

    #[test]
    fn download_verified_discards_file_on_size_mismatch() {
        let (result, kept) = run_download_verified(&run_file(3, Some(ABC_SHA256)), b"abcd");
        assert!(result.unwrap_err().to_string().contains("Size mismatch"));
        assert!(kept.is_none(), "mismatching file must be deleted");
    }

    #[test]
    fn checked_run_relative_path_rejects_escaping_paths() {
        assert!(checked_run_relative_path("../evil.txt").is_err());
        assert!(checked_run_relative_path("/etc/passwd").is_err());
        assert!(checked_run_relative_path("a/../../evil.txt").is_err());
        assert!(checked_run_relative_path("").is_err());
    }

    #[test]
    fn checked_run_relative_path_accepts_nested_relative_paths() {
        assert_eq!(
            checked_run_relative_path("output/result.txt").unwrap(),
            PathBuf::from("output/result.txt")
        );
    }

    #[test]
    fn ensure_single_path_component_accepts_plain_names() {
        assert!(ensure_single_path_component("e2e-vault", "vault name").is_ok());
        assert!(ensure_single_path_component("happy-river", "run name").is_ok());
    }

    #[test]
    fn ensure_single_path_component_rejects_separators_and_traversal() {
        assert!(ensure_single_path_component("a/b", "vault name").is_err());
        assert!(ensure_single_path_component("..", "run name").is_err());
        assert!(ensure_single_path_component("../evil", "run name").is_err());
        assert!(ensure_single_path_component("/abs", "vault name").is_err());
        assert!(ensure_single_path_component("", "vault name").is_err());
    }
}
