# Capsula Codebase Review Report

**Date:** 2026-07-02
**Scope:** All 16 workspace crates at commit `f032d2b`, plus workspace configuration (Cargo.toml, Dockerfile, compose.yaml, CI workflows).
**Method:** Full manual source review of every crate, cross-checked with `cargo clippy --workspace --all-targets --all-features` (clean) and `cargo test --workspace` (all green except `capsula-server` integration tests, which require a Docker socket for testcontainers and could not run in the review environment — an environmental limitation, not a code failure).

All High findings and most Medium findings were verified directly against the source; file:line references point at the offending code.

---

## Summary

| Severity | Count | Themes |
|----------|-------|--------|
| High     | 8     | Exit-code loss, fail-open safety gates, unauthenticated server, path escape in file capture, remote panic |
| Medium   | 17    | TOCTOU races, silent failures, unbounded memory use, API drift, broken pagination |
| Low      | 20+   | Dead code, error-handling smells, encoding edge cases, test gaps |

The core capture pipeline is well-designed (typed hooks, no shell injection anywhere, correct pipe draining, secret redaction in the Slack hook), but two systemic problems recur:

1. **Failures fail open or fail silent.** Hook errors bypass abort gates, the CLI exits 0 on child failure and on abort, the server returns HTTP 200 for errors, and Slack API failures are recorded as success.
2. **The newer server component ships without an authentication story** while ingesting data that by design can contain secrets (env vars, stdout, file contents).

---

## High severity

### H1. `capsula run` never propagates the wrapped command's exit code — always exits 0 (bug)

`crates/capsula-cli/src/main.rs:314-334` — `run.exec()` returns `Ok(RunOutput { exit_code, .. })` even for non-zero exits (`capsula-core/src/run.rs:142-155` deliberately maps signals to `128+s` and returns `Ok`). The CLI logs the exit code, writes it to `command.json`, and falls through to `Ok(())`.

**Impact:** `capsula run cargo test && ./deploy.sh` deploys even when tests fail; any CI step wrapped in `capsula run` always reports success. For a command-wrapper this is the single most surprising behavior. The integration tests (`tests/cli_integration.rs`) only assert `success()` and never cover a failing child, so nothing catches it.

**Fix:** after post-hooks, `std::process::exit(run_output.exit_code)` (or return a typed exit status through `main`).

### H2. Safety gates fail open: a hook *error* never aborts the run (security-control bypass)

`crates/capsula-orchestration/src/hooks.rs:94-105` — hook errors are mapped to `(json, false)`; `abort_requested()` only exists on the `Ok` path. All safety gates (`allow_dirty=false`, `require_pushed=true` in the git hook; `abort_on_failure=true` in the command hook) are signaled via `Captured::abort_requested()`.

**Impact:** if `GitHook::run` errors — e.g. the lightweight tag collides with an existing run name (`capsula-capture-git-repo/src/lib.rs:171-177`, `force=false`), an artifact-dir creation failure, or a permissions error — the error is recorded as non-fatal and the command runs anyway against a dirty/unpushed repo. Same for `capture-command` guards: if the guard binary is missing, the run proceeds. Gates that exist to block execution must fail closed (or at minimum, abort-capable hooks should be marked so their errors are fatal).

### H3. Abort requests exit 0, and `run-start` ignores them entirely (bug, compounds H2)

`crates/capsula-cli/src/main.rs:309-312` — when a pre-run hook requests abort, the CLI logs an error and `return Ok(())` (exit 0), so a calling script cannot distinguish "aborted, command never ran" from "ran successfully". In `run-start` (`main.rs:345-347`) the abort is downgraded to a `warn!` and the run name is still printed — the dirty-repo gate is a no-op for the start/end flow.

### H4. Server has no authentication or authorization on any endpoint (security)

`crates/capsula-server/src/lib.rs:129-151` — `build_app` wires all routes with no auth middleware, no token check, and no TLS; `compose.yaml` binds `CAPSULA_HOST=0.0.0.0` and publishes the port. Anyone with network access can create runs, upload files (100 MB per request, unlimited requests), and read all captured data — which by design includes stdout/stderr, captured environment variables, and file contents from users' machines.

**Impact:** direct data-exposure channel for secrets that capture hooks collect. Even a single shared bearer token checked in middleware would materially change the exposure. At minimum this constraint must be prominently documented ("deploy only behind an authenticating reverse proxy").

### H5. Unauthenticated disk exhaustion: uploaded blobs are persisted even without a valid `run_id` (security)

`crates/capsula-server/src/lib.rs:1067-1078` vs `1080` — the file is written to `storage/{hash[0..2]}/{hash}` *before* the `run_id` check; when `run_id` is absent (or references no run, in which case the later insert fails) the blob stays on disk with no DB reference and no GC. `tests/api_tests.rs:262-282` confirms uploads without `run_id` return `"status":"ok"`.

**Impact:** combined with H4, an attacker can fill the disk with unique-content uploads. Validate `run_id` against the DB before writing any bytes.

### H6. Remote panic via `GET /runs?limit=0` (bug/DoS)

`crates/capsula-server/src/lib.rs:322` and `335`:

```rust
let page = params.offset.unwrap_or(0) / params.limit.unwrap_or(50) + 1;
...
let total_pages = (total_count + limit - 1) / limit;
```

`limit` is attacker-controlled; `?limit=0` is an integer division by zero that panics the handler task (no `CatchPanicLayer` is installed, so the connection is torn down). Negative limits flow into SQL `LIMIT $2` and produce Postgres errors. Clamp `limit` to `1..=MAX` as `query.rs` already does for search.

### H7. `capture-file` glob escapes the project root and follows symlinks — arbitrary file capture (security)

`crates/capsula-capture-file/src/lib.rs:106-108`:

```rust
fn build_glob_pattern(base: &Path, pattern: &str) -> String {
    base.join(pattern).to_string_lossy().replace('\\', "/")
}
```

`Path::join` with an absolute pattern discards `base` entirely, so `glob = "/home/user/.ssh/*"` or `glob = "../../etc/*"` is honored verbatim — no canonicalization or containment check. `path.is_file()` (line 123) and `std::fs::copy` (line 163) both follow symlinks, so a symlink committed inside a cloned repo can pull `~/.aws/credentials` into `.capsula/`. Running `capsula run` in a freshly cloned repo executes that repo's committed `capsula.toml`; combined with `capsula push` or the Slack hook, captured secrets leave the machine. `mode = "move"` is worse — it *removes* the matched file from its original location (`fs::rename`, line 165). The same absolute-path-join escape exists independently in `capsula-notify-slack`'s `attachment_globs` (`lib.rs:120-121`), which uploads matched files to an attacker-chosen Slack workspace (see M12).

**Fix:** canonicalize matches and require containment in `project_root` (or an explicit allowlist), and use `symlink_metadata` to skip symlinks by default.

### H8. Byte-index slicing panic in `capsula list` on multibyte commands (bug)

`crates/capsula-cli/src/main.rs:181-182`:

```rust
let command_truncated = if command_display.len() > command_width {
    format!("{}...", &command_display[..command_width - 3])
```

`len()` is bytes and `[..N]` is a byte slice; a recorded command containing non-ASCII (Japanese filename, emoji) crossing the boundary panics with "byte index … is not a char boundary", making `capsula list` permanently crash for that vault. The workspace allows `indexing_slicing` (root `Cargo.toml:79`), so clippy doesn't flag it. Truncate on `char_indices()` (or a display-width crate).

---

## Medium severity

### Core / CLI

**M1. Run-directory collision retry loop can never succeed.** `capsula-core/src/run.rs:62-87` — the retry loop recomputes `gen_run_dir(&vault_dir)` from `self.id` and `self.name`, neither of which changes between attempts, so every retry produces the identical path; on a real collision it just sleeps and fails after `max_retries`. Additionally there is a TOCTOU between the `exists()` check (line 67) and `create_dir_all` (line 90): `create_dir_all` succeeds on an existing directory, so two concurrent runs that collide silently share one run directory and overwrite each other's `metadata.json`/`pre-run.json`. Fix both by using `fs::create_dir` and treating `ErrorKind::AlreadyExists` as the collision signal, regenerating the name on retry.

**M2. Broken pipe on the console discards the whole run record.** `capsula-core/src/run.rs:223, 240, 252-257` — the capture threads tee child output to capsula's stdout/stderr with `console.write_all(...)?`. Rust ignores SIGPIPE, so `capsula run cmd | head` makes the tee fail with EPIPE after `head` exits; the `??` propagates it and `exec()` returns `Err` even though the child already completed — exit code, output, and duration are all lost and no `command.json` is written. Console-write failures should stop echoing, not poison the capture.

**M3. Unbounded in-memory buffering of child stdout/stderr.** `capsula-core/src/run.rs:214-241` — entire stdout/stderr accumulate in `Vec<u8>`s and are then embedded in `command.json`. A verbose or long-running job (the tool's target audience) can OOM capsula and lose the run. There is also no redaction story: any secret the command prints is persisted in plaintext and uploaded by `capsula push`. Needs a size cap / spill-to-file, and a documented redaction stance. The same unbounded `Command::output()` pattern exists in `capture-command` (`lib.rs:81-92`), which additionally has no timeout — a hung guard command wedges `capsula run` forever.

**M4. `.gitignore` creation skipped whenever the vault dir already exists.** `capsula-core/src/run.rs:278-281` — the guard checks the *directory*, not the ignore file. If the user pre-created the vault dir, or a prior run crashed between `create_dir_all` and the `.gitignore` write, no later run ever writes it — and captured env vars/files/output become committable. Make the `.gitignore` write idempotent by checking for the file.

**M5. `resolve_relative` permits config-driven path escape and has split semantics.** `capsula-core/src/util.rs:21-27` — absolute paths pass through unvalidated while relative paths are canonicalized (and must exist); `../../..` resolves outside `project_root` with no containment check. Used by the git and command hooks to pick their working directory — path-traversal-by-config, same class as H7.

**M6. Stale `GIT_HASH` in `--version`.** `capsula-cli/build.rs:22` — only `.git/HEAD` is registered with `rerun-if-changed`; HEAD doesn't change on new commits to the same branch, so the embedded hash goes stale until a branch switch. For an auditing tool a wrong-but-plausible hash is worse than none. Register the resolved ref file and `.git/packed-refs` too.

**M7. TOML datetimes leak `$__toml_private_datetime` into hook configs.** `capsula-config/src/lib.rs:96-101` — a hook option written as a bare TOML datetime deserializes into the flattened `serde_json::Value` as `{"$__toml_private_datetime": "..."}`, producing baffling hook-config errors and polluting `__meta.config` in artifacts. Normalize or reject datetime values at parse time.

### Server / client

**M8. JSON API returns errors and not-found as HTTP 200.** `capsula-server/src/lib.rs:567-574, 892-905, 624-655, 927-931` — `list_runs`, `get_run` (returns `"status":"not_found"` with 200), `search_runs`, and every `upload_files` error path respond `Json(...)` with the default 200. The bundled client gates on `status().is_success()` (`capsula-client/src/lib.rs:44,60,119`), so server-side failures pass the check and surface as misleading deserialization errors — or, for `vault_exists`, silently wrong answers. Adopt a typed error → `(StatusCode, Json)` response for all handlers.

**M9. Multipart stream errors are swallowed and reported as success.** `capsula-server/src/lib.rs:940` — `while let Ok(Some(field)) = multipart.next_field().await` ends the loop on error (malformed multipart, body-limit hit, client disconnect) and falls through to `"status": "ok"` with partial state committed. A truncated upload is indistinguishable from a complete one.

**M10. Non-atomic content-addressed writes can permanently poison the dedup store.** `capsula-server/src/lib.rs:1067-1078` — `tokio::fs::write` is create+truncate, not atomic; a crash mid-write leaves a truncated blob at the hash path, and every future upload of that content "deduplicates" against the corrupt blob (the hash is never re-verified on read). Write to a temp file and `rename`.

**M11. Upload semantics depend on multipart field order; re-uploads leave stale hook rows.** `capsula-server/src/lib.rs:1080-1096` — a `file` part received before `run_id` is stored to disk but never recorded against the run (silent data loss with a 200); `path` fields must immediately precede their file part. And hooks are upserted by `(run_id, phase, hook_index)` with no delete of higher indices (`lib.rs:1180-1198`), so re-uploading a run with fewer hooks yields a mixed record from two uploads.

**M12. Slack hook: `ok:false` API responses treated as success; attachment globs share H7's escape.** `capsula-notify-slack/src/lib.rs:291-303` — `send_simple_message` checks only HTTP status, but Slack returns HTTP 200 with `{"ok":false,"error":"invalid_auth"}` for most failures, so a bad token or wrong channel is recorded as "sent successfully" (the file-upload path *does* check `ok` — lines 202-224 — showing the check was known to be needed). Attachment globs use the same `base_dir.join(pattern)` absolute-path escape as H7 (`lib.rs:120-121`) and `std::fs::read` each attachment fully into memory with no size cap (`lib.rs:539`).

**M13. `capture-file` hash/copy TOCTOU and flat-destination collisions.** `capsula-capture-file/src/lib.rs:140-167` — the file is read twice (hash, then copy); a concurrent writer yields a recorded hash that doesn't match the archived bytes, defeating the audit guarantee. And `artifact_dir.join(file_name)` flattens paths, so `**/config.json` matching `a/config.json` and `b/config.json` silently overwrites — while the JSON output claims both were captured with incompatible hashes. Hash the copied bytes, and preserve relative paths (or uniquify) in the destination.

**M14. Git "dirty" patch omits staged changes and untracked content.** `capsula-capture-git-repo/src/lib.rs:219-224` — `is_dirty` uses `repo.statuses` (includes index-vs-HEAD), but the saved patch is `diff_index_to_workdir` only; `git add file && capsula run ...` records a dirty run whose patch contains none of the staged changes. `include_untracked(true)` lists untracked entries without their content — `show_untracked_content(true)` is also needed for reconstructibility.

**M15. `capture-env` logs captured values in plaintext.** `capsula-capture-env/src/lib.rs:55-63` — the hook captures a *named* variable (typically `AWS_...`, `..._API_KEY`) and echoes its value into `debug!` logs, plus stores it unredacted with no hash-only option — despite the workspace already having `SecretString` (`capsula-notify-slack/src/secret.rs`) for exactly this. At minimum drop the value from the log line.

**M16. No `deny_unknown_fields` on any hook config.** e.g. `capsula-capture-git-repo/src/lib.rs:20-33`, `capsula-capture-file/src/lib.rs:15-21` — `alow_dirty = false` (typo) deserializes fine and the gate silently never engages; `CwdHook::from_config` (`capsula-capture-cwd/src/lib.rs:43-50`) accepts arbitrary JSON without deserializing at all. For security-relevant knobs, typos must be fatal config errors.

**M17. Shared API-types crate has already drifted from the server's models.** `capsula-api-types/src/lib.rs:85-116` vs `capsula-server/src/models.rs:83-138` — `SearchRunsRequest`, `HookFilter`, `IncludeField`, `SortOrder` are defined in both places; the shared crate's `from`/`to` are `Option<String>` while the server's are `Option<DateTime<Utc>>`. The crate whose purpose is compile-time client/server compatibility no longer guarantees it; the server should consume `capsula-api-types` directly.

---

## Low severity / code smells

### Correctness edges

- **L1.** HTML pagination is dead: templates link `?page=N` (`capsula-server/templates/runs.html:64,70`) but `ListRunsQuery` has no `page` field (`models.rs:35-40`) — Next/Previous always re-render page 1, so the web UI can never show runs beyond the first 50.
- **L2.** `GET /api/v1/runs` has no limit clamp (`lib.rs:543`), unlike search's `MAX_LIMIT = 1_000`; `?limit=9223372036854775807` dumps the whole table with all stdout/stderr blobs.
- **L3.** `capsula run` can't run commands whose first token starts with `-` (`main.rs:49-52`, no `allow_hyphen_values`); `capsula run --version prog` silently prints capsula's own version.
- **L4.** TUI detection scans raw args for the literal token `tui` (`main.rs:568-579`) and routes all tracing to `io::sink` — `capsula run make tui` fails with exit 1 and zero output, and `capsula tui` itself errors silently when `capsula.toml` is missing. Errors should go to stderr directly; detect the TUI subcommand via clap.
- **L5.** `RUST_BACKTRACE=0` counts as "verbose" (`main.rs:595-596` checks `is_ok()`, not the value).
- **L6.** Move-mode `fs::rename` fails across filesystems with `EXDEV` (`capsula-capture-file/src/lib.rs:165`); needs copy+delete fallback. A broad glob can also capture the vault's own `.capsula` directory into itself.
- **L7.** Non-UTF-8 handling: run/project paths are persisted via `to_string_lossy` (`capsula-core/src/run.rs:113-186`), so `CAPSULA_RUN_DIRECTORY` can point at a nonexistent path; `capture-env`'s `.ok()` conflates "unset" with "non-UTF-8 value"; `capture-command` conflates signal death with exit code -1 (`lib.rs:96`, `ExitStatusExt::signal()` is available).
- **L8.** `CAPSULA_RUN_COMMAND` fallback plain-joins args when `shlex::try_join` fails (`run.rs:175-176`), silently mis-tokenizing quoted args in the provenance record.
- **L9.** TUI end-run re-resolves the run directory by generated *name* and picks the newest match (`capsula-tui/src/app.rs:200`, `vault.rs:169-231`) instead of remembering the directory it created — the name space is small, so a collision can finalize the wrong run. Hooks also run synchronously on the render thread (`capsula-tui/src/lib.rs:69-72`), freezing input.
- **L10.** Duplicate-key detection by message substring: server `e.to_string().contains("duplicate key")` (`lib.rs:808`) and CLI push `contains("already exists")` (`main.rs:451`). The orchestration re-push path depends on that 409 (`push.rs:72`), so a message change silently converts "already pushed" into a hard failure — and an unrelated error containing "already exists" is miscounted as a skip with exit 0. Use `is_unique_violation()` and typed errors.

### Hygiene / design smells

- **L11.** Internal DB error strings (schema/constraint names) are returned to unauthenticated clients throughout the server (`lib.rs:458-462, 481, 531, 571, 817, 902`, all upload errors).
- **L12.** N+1 queries in `search_runs` (`lib.rs:668-737`) — up to two extra sequential queries per matched run (×1,000); `bind_values` cloned twice per request.
- **L13.** Whole bodies buffered in memory server-side: `field.bytes()` up to 100 MB per file (`lib.rs:1032`); downloads `tokio::fs::read` the entire blob instead of streaming (`lib.rs:1305`) — `tokio-util` is declared, presumably for `ReaderStream`, but unused. Client mirrors this (`capsula-client/src/lib.rs:101`) and uses the default reqwest blocking client whose 30 s total timeout will abort large uploads (`lib.rs:35`).
- **L14.** `push_single_run` constructs its own `reqwest::blocking::Client` (`capsula-orchestration/src/push.rs:68`) despite receiving `&CapsulaClient` — duplicated HTTP plumbing that will diverge the moment auth/timeouts are added.
- **L15.** Dead error variants across crates (never constructed): `RegistryError::HookTypeNotFound`/`HookCreationFailed`, `ConfigError::Invalid`, `FileHookError::{FileNotFound, NoFilesMatched, RunDirNotSet, ReadError, HashError}`, `EnvVarHookError::{VariableNotFound, InvalidUtf8}`, `CommandHookError::{InvalidUtf8, NonZeroExit}`, `GitHookError::RunDirNotSpecified`. Also `IncludeField::Metadata` is accepted but never honored by `search_runs`, and `capsula-core` declares an unused `clap` dependency.
- **L16.** `Run`'s type-state guarantee is bypassable: all fields `pub` plus derived `Deserialize` (`run.rs:13-20`) lets anyone construct a "prepared" `Run<PathBuf>` with a nonexistent dir; the custom `Serialize` impls don't round-trip with the derived `Deserialize`.
- **L17.** Architecture drift: `capsula-registry` (documented as Tier 2) depends on all seven hook crates, inverting the documented layering; CLAUDE.md still says 11 crates and doesn't mention server/client/tui/orchestration/api-types at all. Registry error messages list available hook types in nondeterministic HashMap order (`registry/src/lib.rs:117-119`).
- **L18.** `capture-machine` bakes the typo `vender_id` into the persisted JSON schema (`capsula-capture-machine/src/lib.rs:23`) — fix with `#[serde(rename)]` before more data accumulates.
- **L19.** `capture-file`: one unreadable file aborts the entire hook (`lib.rs:128` collects into `Result`), contradicting the documented partial-success philosophy, while `filter_map(Result::ok)` on line 122 silently drops `GlobError`s; a source path with no `file_name()` is reported as "Invalid run directory" (`lib.rs:157-159`). Blanket `replace('\\', "/")` corrupts legal-backslash Unix filenames (`lib.rs:107`) — should be Windows-only.
- **L20.** Slack hook: `Hook<PreRun>`/`Hook<PostRun>` impl blocks duplicated verbatim (`lib.rs:576-655`); the raw file-upload POST response is never status-checked (`lib.rs:245-249`); overlapping attachment patterns double-count files against the 10-file cap.
- **L21.** Test gaps that map to the High findings: no test runs a failing child command (would catch H1), none lists a multibyte command (H8), none covers `runs_page` pagination or `?limit=0` (H6, L1); `capsula-client` declares `wiremock` but has no HTTP tests; `capsula-config`'s integration test silently passes if its input file is missing; the CI/local toolchain drift (crates require rustc 1.95, `rust-toolchain.toml` just says `stable`) means older stables fail to build with a confusing message.
- **L22.** Supply chain: workflows are otherwise fully SHA-pinned, but `pip install zensical` is unpinned in the docs deploy workflow (`.github/workflows/docs.yaml:26`) which holds `pages: write` + `id-token: write`.

---

## What's done well

- **No shell injection surface anywhere:** all subprocess invocations (core exec, command hook, git operations) use argv arrays, never a shell.
- **Slack token handling:** `SecretString` redacts in both `Debug` and `Serialize`, so the token lands as `"***"` in `__meta.config` — with a test pinning it.
- **Pipe handling:** stdout/stderr are drained concurrently before `wait()`, correctly avoiding the classic pipe-buffer deadlock; Unix signal deaths map to `128+signal`.
- **Server input hardening where it exists:** all SQL uses bound parameters (the dynamic search builder interpolates only server-generated fragments and clamped numerics); `sanitize_relative_path` is thorough and well-tested; `Content-Disposition` filenames are RFC 5987-encoded; askama templates rely on default HTML escaping with no `|safe` bypasses.
- **CI hygiene:** actions SHA-pinned with `persist-credentials: false`, zizmor on every PR, digest-pinned Docker images, non-root runtime user in the Dockerfile.

---

## Suggested priorities

1. **Exit codes & fail-closed gates (H1, H2, H3):** small diffs, immediately fixes the tool's core contract.
2. **Server exposure (H4, H5, H6, M8-M11):** add auth (even a shared token), clamp `limit`, validate `run_id` before writing blobs, return real status codes.
3. **Config-driven path escapes (H7, M5, M12):** containment checks + symlink policy in `capture-file`, `notify-slack`, and `resolve_relative`.
4. **Panic and data-loss edges (H8, M1-M4):** char-boundary truncation, collision-safe run-dir creation, EPIPE-tolerant teeing, idempotent `.gitignore`.
5. **Drift cleanup (M17, L15, L17):** de-duplicate API types, delete dead error variants, refresh CLAUDE.md to the 16-crate reality.
