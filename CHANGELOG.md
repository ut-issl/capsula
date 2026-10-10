# Changelog

Release notes for Capsula. Each release section becomes the body of its GitHub Release.
Add a `## v<version>` section before running the release workflow (see `RELEASING.md`).
The `rs-v*` sections are the early releases of the Rust rewrite.
Releases of the earlier Python implementation (v0.8.0 and before) are listed on [GitHub Releases](https://github.com/ut-issl/capsula/releases).

## v0.15.1

This patch release addresses a `rustls` security advisory and refreshes Capsula's development tooling.

### Security

#### Update `rustls` to address a security advisory

Update `rustls` to 0.23.45 to address [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285).

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1247>_

### Development and CI

- Update the development and CI toolchain to Rust 1.98.1, including the Rust Docker image (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1241>, <https://github.com/ut-issl/capsula/pull/1248>_)
- Optimize the development setup for Amp orbs (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1217>_)
- Update `zizmorcore/zizmor-action` to v0.6.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1249>_)
- Update `taiki-e/install-action` from v2.87.4 through v2.87.10 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1240>, <https://github.com/ut-issl/capsula/pull/1242>, <https://github.com/ut-issl/capsula/pull/1243>, <https://github.com/ut-issl/capsula/pull/1244>, <https://github.com/ut-issl/capsula/pull/1246>, <https://github.com/ut-issl/capsula/pull/1251>, <https://github.com/ut-issl/capsula/pull/1252>_)

### Internal Changes

- Refresh the Cargo lock file (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1245>_)
- Bump the workspace version to v0.15.1 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1250>_)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.15.0...v0.15.1>

## v0.15.0

### New Features

#### Authenticate server requests with configurable HTTP headers

Capsula can now attach custom HTTP headers to requests made to a configured server. This supports deployments protected by bearer tokens, API keys, reverse proxies, and identity-aware access gateways.

Change `server` from a URL string to a table and define headers using literal values, environment variables, or command output:

```toml
[server]
url = "https://capsula.example.com"

[server.headers]
Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }
X-API-Key = { command = "pass show services/capsula/api-key" }
```

The existing string form remains supported when no headers are needed:

```toml
server = "https://capsula.example.com"
```

Environment variables are resolved after the configured dotenv file is loaded. Header commands run from the project root, and trailing newlines are removed from their output.

Configured headers are used by server-facing commands such as `push`, `pull`, and `vaults list`. Credential handling includes several safeguards: header values are marked sensitive, redirects are not followed, and credentials are rejected when a `--server` or `CAPSULA_SERVER_URL` override points to a different origin than the configured server.

_by @shunsuke-shimomura in <https://github.com/ut-issl/capsula/pull/1160>_

#### Add the `capsula pull` command

A new `capsula pull` command restores a server-side run into a local vault by run ID:

```console
capsula pull 01HQXYZ...
```

Use `--vault <name>` when the run belongs to a vault other than the one in `capsula.toml`, and `--force` to refresh a run that was previously pulled:

```console
capsula pull 01HQXYZ... --vault another-vault --force
```

Capsula reconstructs the run directory and `_capsula` metadata, downloads captured artifacts, and verifies their recorded sizes and SHA-256 hashes when hashes are available. Downloads are assembled in a temporary directory and moved into place only after completion, preventing interrupted pulls from appearing as valid runs.

Pulled runs carry an `_capsula/pulled.json` origin marker. Because server data cannot reproduce every byte of the original local metadata, pulled runs are treated as lossy reconstructions: `--force` can replace only an earlier pulled copy, never a locally produced run, and Capsula refuses to push pulled runs back to the server.

_by @shunsuke-shimomura in <https://github.com/ut-issl/capsula/pull/1178>_

### Development and CI

- Add dependency vulnerability auditing to CI and `just lint` (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1213>_)
- Make local linting deny build warnings, matching CI behavior (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1211>_)
- Replace markdownlint with the Rust-based `rumdl` Markdown linter (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1197>_)
- Update the pinned development and CI toolchain to Rust 1.98 while retaining Rust 1.95 as the minimum supported Rust version (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1209>, <https://github.com/ut-issl/capsula/pull/1233>_)

### Internal Changes

- Update `ulid` to v3 across the workspace (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1152>_)
- Update `sql-json-path` to 0.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1232>_)
- Update Rust dependencies including `serde_json` 1.0.151, `clap` 4.6.3, and `tokio` 1.53.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1161>, <https://github.com/ut-issl/capsula/pull/1164>, <https://github.com/ut-issl/capsula/pull/1165>_)
- Update GitHub Actions for checkout, Python setup, Docker login/build/QEMU, Rust caching, Pages deployment, typo checking, and workflow security scanning (_by @renovate[bot] in multiple pull requests_)
- Update `taiki-e/install-action` from v2.84.0 through v2.87.3 (_by @renovate[bot] in multiple pull requests_)
- Refresh Rust and PostgreSQL container image digests and Cargo lock files (_by @renovate[bot] in multiple pull requests_)
- Bump the workspace version to v0.15.0 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1239>_)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.14.0...v0.15.0>

## v0.14.0

### New Features

#### `capture-yaml` hook

A new built-in `capture-yaml` hook parses a YAML file into structured, queryable run output. The hook is available in both pre-run and post-run phases.

```toml
[[pre-run.hooks]]
id = "capture-yaml"
path = "config/sat1/orbit.yaml"
```

Each hook instance captures one file and stores its JSON-compatible representation under `content`. Relative paths are resolved from the project root, absolute paths are also accepted, and the configured path remains available under `__meta.config.path`. Add multiple hook entries to capture multiple files.

Plain, JSON-representable YAML is recommended. Multiple-document streams and unsupported YAML constructs produce a hook error, while other hooks continue to run. See the `capture-yaml` hook documentation for detailed conversion behavior and parser limitations.

_by @shunsuke-shimomura in <https://github.com/ut-issl/capsula/pull/1149>_

### Release Engineering

#### Restore full workspace publishing

The release workflow once again publishes the complete Cargo workspace to crates.io. Publishing now uses `cargo publish --workspace --locked` so releases are built from the locked dependency graph, and package metadata points to the correct repository URL.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1133>, <https://github.com/ut-issl/capsula/pull/1134>, <https://github.com/ut-issl/capsula/pull/1135>_

### Internal Changes

- Add `.env.keys` to `.gitignore` to avoid tracking dotenvx encryption keys (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1143>_)
- Refactor internal APIs with Hawk recommendations (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1144>_)
- Refresh repository guidance in `AGENTS.md` (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1142>_)
- Make Codecov coverage checks informational (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1155>_)
- Add `rust-analyzer` to the Rust toolchain components (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1157>_)
- Update Rust development and CI tooling through Rust 1.97.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1145>, <https://github.com/ut-issl/capsula/pull/1151>_)
- Update `taiki-e/install-action` from v2.83.1 through v2.83.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1137>, <https://github.com/ut-issl/capsula/pull/1139>, <https://github.com/ut-issl/capsula/pull/1154>, <https://github.com/ut-issl/capsula/pull/1156>_)
- Update `zizmorcore/zizmor-action` to v0.6.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1150>_)
- Update PostgreSQL 18 Docker image digests (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1131>, <https://github.com/ut-issl/capsula/pull/1140>_)
- Refresh the Cargo lock file (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/1141>_)
- Bump the workspace version to v0.14.0 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1158>_)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.13.0...v0.14.0>

## v0.13.0

### Breaking Changes

#### Propagate the wrapped command's exit code from `capsula run`

`capsula run` now exits with the wrapped command's exit code after post-run hooks finish. Command results are still persisted before Capsula exits, but scripts and CI jobs will now correctly observe a failed wrapped command as a failed `capsula run` invocation.

Pre-run hook aborts now use the dedicated exit code `125`. The `run-start` command also honors abort requests and no longer prints a run name when a hook aborts the run.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1094>, <https://github.com/ut-issl/capsula/pull/1095>_

#### Raise the minimum supported Rust version to 1.95

The minimum supported Rust version (MSRV) is now Rust 1.95, enabling the upgrade to `sysinfo` 0.39.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/964>, <https://github.com/ut-issl/capsula/pull/965>_

### New Features

#### Add `capture-json` and `capture-toml` hooks

Two new built-in hooks parse a single JSON or TOML file into queryable run parameters. Both hooks are available in pre-run and post-run phases and emit the configured path as `file` and the parsed value as `content`.

```toml
[[pre-run.hooks]]
id = "capture-json"
path = "config/orbit.json"

[[pre-run.hooks]]
id = "capture-toml"
path = "config/simulation.toml"
```

Use multiple hook entries to capture multiple files. Paths are resolved relative to the project root, while the configured path is preserved in the output. TOML datetime values are represented as strings, and non-finite TOML floats are represented as `null`.

_by @shunsuke-shimomura in <https://github.com/ut-issl/capsula/pull/1016>, <https://github.com/ut-issl/capsula/pull/1018>_

#### Publish Capsula Server container images to GHCR

The release workflow now publishes multi-platform Capsula Server images for `linux/amd64` and `linux/arm64` to `ghcr.io/ut-issl/capsula-server`. Releases publish full-version, minor-version, major-version, and `latest` tags after the crates.io packages have been published.

_by @shunsuke-shimomura in <https://github.com/ut-issl/capsula/pull/1102>_

### Server Improvements

#### Support multiple instances of the same hook

Server run outputs are now keyed by each hook's position (`hook_index`) instead of `hook_id`. This allows a run to contain multiple instances of the same hook—even instances with identical configurations—without rows colliding during upload.

Run responses now expose `hook_index` in hook metadata and return hooks in their original configuration order. Upload and query models have also been separated so the server remains authoritative for hook indices.

The database migration runs automatically and backfills indices for existing outputs.

_by @shunsuke-shimomura in <https://github.com/ut-issl/capsula/pull/1036>, <https://github.com/ut-issl/capsula/pull/1099>_

### Fixes

#### Reject unknown hook configuration fields

All built-in hook configurations now reject unknown fields. Configuration typos that were previously ignored are reported as errors instead, including for hooks with otherwise empty configurations.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1096>_

#### Prevent unintended TUI actions on Windows

The TUI now processes only key-press events and ignores key-release and repeat events. This fixes an issue where the Enter key release from launching `capsula tui` could immediately activate the focused action on Windows.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1092>_

### Documentation

- Document Capsula Server in the README (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1025>_)

### Internal Changes

- Improve Clippy test lint configuration across the workspace (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/968>_)
- Use crate-local SQLx query metadata (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1129>_)
- Add and update CODEOWNERS (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1012>, <https://github.com/ut-issl/capsula/pull/1101>_)
- Simplify typo and dependency-vetting configuration (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1031>, <https://github.com/ut-issl/capsula/pull/1032>_)
- Delay Docker Hub digest updates until images are stable (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1043>_)
- Freeze pre-commit hook revisions (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1060>_)
- Pin the Rust toolchain used by CI (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1108>_)
- Update Rust dependencies, including `shlex` 2, `git2` 0.21, `sqlx` 0.9, `reqwest` 0.13.4, `sysinfo` 0.39.5, and `anyhow` 1.0.103 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/979>, <https://github.com/ut-issl/capsula/pull/983>, <https://github.com/ut-issl/capsula/pull/989>, <https://github.com/ut-issl/capsula/pull/995>, <https://github.com/ut-issl/capsula/pull/1001>, <https://github.com/ut-issl/capsula/pull/1049>, <https://github.com/ut-issl/capsula/pull/1055>_)
- Update the CI Rust image and toolchain through Rust 1.97 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/970>, <https://github.com/ut-issl/capsula/pull/1002>, <https://github.com/ut-issl/capsula/pull/1022>, <https://github.com/ut-issl/capsula/pull/1023>, <https://github.com/ut-issl/capsula/pull/1035>, <https://github.com/ut-issl/capsula/pull/1042>, <https://github.com/ut-issl/capsula/pull/1051>, <https://github.com/ut-issl/capsula/pull/1106>, <https://github.com/ut-issl/capsula/pull/1128>_)
- Update GitHub Actions, Docker actions, PostgreSQL image digests, and lock files (_by @renovate[bot] in multiple pull requests_)
- Update Codecov action configuration (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1009>_)
- Bump version to v0.13.0 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/1120>_)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.12.1...v0.13.0>

## v0.12.1

### Security

#### Wrap Slack bot token in `SecretString` to prevent leaks

The `notify-slack` hook now stores its bot token in a `SecretString` wrapper so the secret is redacted from `Debug` output and serialized config dumps. This reduces the chance of leaking the token via logs or captured run metadata.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/889>_

#### Harden server: redact DB URL, encode `Content-Disposition`, validate captured-file path

Several server-side hardening fixes:

- The database URL is redacted from logs and error messages.
- The `Content-Disposition` header is properly encoded to prevent header-injection via filenames.
- Captured-file paths are validated to prevent path-traversal when serving artifacts.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/890>_

#### Harden GitHub Actions workflows

GitHub Actions workflows have been hardened (least-privilege permissions, etc.), and the repository now opts into GitHub Advanced Security.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/955>, <https://github.com/ut-issl/capsula/pull/956>_

#### Bump `rustls-webpki` from 0.103.12 to 0.103.13

_by @dependabot in <https://github.com/ut-issl/capsula/pull/915>_

### Fixes

#### Replace framework-invariant panics with error values

Several places that previously panicked on internal framework invariants now return error values instead, so unexpected states are surfaced as recoverable errors rather than aborting the process.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/894>_

#### Correctness and polish across crates

Small correctness fixes and polish across multiple crates.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/896>_

#### Fix release tag authentication

The release workflow's tag step now authenticates correctly, unblocking automated tagging during releases.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/963>_

### Refactoring

- Unify standard pre/post registry builders (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/891>_)
- Dedupe `PreRun`/`PostRun` implementations in `notify-slack` via shared helpers (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/892>_)
- Share helpers across crates in `capsula-core` (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/893>_)
- Dedup server queries and bound `OFFSET` (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/895>_)
- Make TUI `ui::draw` pure by returning hit-test rects (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/897>_)
- Prune dead code and abandoned stubs (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/898>_)
- Fix issues with the new Clippy lints (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/888>_)

### Documentation

#### Update output directory structure docs for per-hook artifact dirs

Documentation has been updated to reflect the per-hook artifact directory layout introduced in v0.12.0.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/946>_

### Internal Changes

- Update `askama` to 0.16 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/933>_)
- Update Rust crate `assert_cmd` to v2.2.1, v2.2.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/914>, <https://github.com/ut-issl/capsula/pull/960>_)
- Update Rust crate `reqwest` to v0.13.3 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/925>_)
- Update `actions/upload-pages-artifact` to v5 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/887>_)
- Update `crate-ci/typos` to v1.45.1, v1.45.2, v1.46.0, v1.46.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/905>, <https://github.com/ut-issl/capsula/pull/924>, <https://github.com/ut-issl/capsula/pull/934>, <https://github.com/ut-issl/capsula/pull/950>_)
- Update `taiki-e/install-action` to v2.73.0 through v2.77.6 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/881>, <https://github.com/ut-issl/capsula/pull/883>, <https://github.com/ut-issl/capsula/pull/885>, <https://github.com/ut-issl/capsula/pull/886>, <https://github.com/ut-issl/capsula/pull/899>, <https://github.com/ut-issl/capsula/pull/901>, <https://github.com/ut-issl/capsula/pull/902>, <https://github.com/ut-issl/capsula/pull/904>, <https://github.com/ut-issl/capsula/pull/906>, <https://github.com/ut-issl/capsula/pull/907>, <https://github.com/ut-issl/capsula/pull/908>, <https://github.com/ut-issl/capsula/pull/910>, <https://github.com/ut-issl/capsula/pull/911>, <https://github.com/ut-issl/capsula/pull/913>, <https://github.com/ut-issl/capsula/pull/916>, <https://github.com/ut-issl/capsula/pull/917>, <https://github.com/ut-issl/capsula/pull/919>, <https://github.com/ut-issl/capsula/pull/920>, <https://github.com/ut-issl/capsula/pull/921>, <https://github.com/ut-issl/capsula/pull/922>, <https://github.com/ut-issl/capsula/pull/926>, <https://github.com/ut-issl/capsula/pull/928>, <https://github.com/ut-issl/capsula/pull/929>, <https://github.com/ut-issl/capsula/pull/931>, <https://github.com/ut-issl/capsula/pull/935>, <https://github.com/ut-issl/capsula/pull/940>, <https://github.com/ut-issl/capsula/pull/941>, <https://github.com/ut-issl/capsula/pull/944>, <https://github.com/ut-issl/capsula/pull/945>, <https://github.com/ut-issl/capsula/pull/947>, <https://github.com/ut-issl/capsula/pull/951>, <https://github.com/ut-issl/capsula/pull/954>, <https://github.com/ut-issl/capsula/pull/957>, <https://github.com/ut-issl/capsula/pull/958>, <https://github.com/ut-issl/capsula/pull/961>_)
- Update `rust` Docker tag to v1.95.0 and digest updates (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/884>, <https://github.com/ut-issl/capsula/pull/927>, <https://github.com/ut-issl/capsula/pull/937>, <https://github.com/ut-issl/capsula/pull/938>, <https://github.com/ut-issl/capsula/pull/939>, <https://github.com/ut-issl/capsula/pull/952>_)
- Update `postgres:18` Docker digest (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/909>, <https://github.com/ut-issl/capsula/pull/912>, <https://github.com/ut-issl/capsula/pull/936>, <https://github.com/ut-issl/capsula/pull/942>, <https://github.com/ut-issl/capsula/pull/949>, <https://github.com/ut-issl/capsula/pull/953>_)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/882>, <https://github.com/ut-issl/capsula/pull/903>, <https://github.com/ut-issl/capsula/pull/918>, <https://github.com/ut-issl/capsula/pull/923>, <https://github.com/ut-issl/capsula/pull/943>, <https://github.com/ut-issl/capsula/pull/959>_)
- Bump version to v0.12.1 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/962>_)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.12.0...v0.12.1>

## v0.12.0

### Breaking Changes

#### Per-hook artifact directories

Hooks that produce file artifacts (`capture-file`, `capture-git-repo`) now write to dedicated directories under the run directory, named `{phase}-{index}-{hook_id}/` (e.g., `pre-0-capture-file/`, `post-1-capture-git-repo/`). This prevents filename collisions between hooks and provides an extensible structure for future improvements.

Hooks opt in via the new `needs_artifact_dir()` method on the `Hook` trait. The orchestrator creates the directory on demand and passes it via `RuntimeParams`.

New run directory structure:

```text
.capsula/{vault}/{date}/{time-name}/
├── _capsula/
│   ├── metadata.json
│   ├── pre-run.json
│   ├── command.json
│   └── post-run.json
├── pre-0-capture-file/
│   └── output.csv
├── pre-1-capture-git-repo/
│   └── main-repo.patch
├── post-0-capture-file/
│   └── settings.yaml
└── ...
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/863_>

### Improvements

#### Make `capsula push` idempotent to allow retry after interruption

When `capsula push` was interrupted after run metadata was created on the server but before file upload completed, retrying previously failed with a 409 Conflict error. Now the client treats 409 as non-fatal and proceeds to re-upload files and hooks. The server-side hook `INSERT` statements use upsert semantics to prevent duplicate `run_outputs` rows on retry.

A new database migration adds the required `UNIQUE` constraint on `(run_id, phase, hook_id)` to `run_outputs`. The migration runs automatically on server startup — no manual action is required.

Closes #826.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/878_>

#### Embed static CSS into server binary

The server binary now embeds `style.css` via `include_str!()` instead of serving it from the filesystem. The `CAPSULA_STATIC_DIR` environment variable is no longer needed, simplifying server deployment. The `tower-http` dependency has been removed.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/875_>

#### Use localhost for server config

The default server config now uses `localhost` instead of `0.0.0.0`.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/877_>

### Internal Changes

- Improve Renovate config (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/865_>)
- Use SemVer tag in taiki-e/install-action workflow (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/867_>)
- Pin Docker image version (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/869_>)
- Update Swatinem/rust-cache action to v2.9.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/866_>)
- Update taiki-e/install-action action to v2.70.4, v2.71.0, v2.71.1, v2.71.2, v2.71.3, v2.72.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/868,> <https://github.com/ut-issl/capsula/pull/871,> <https://github.com/ut-issl/capsula/pull/872,> <https://github.com/ut-issl/capsula/pull/873,> <https://github.com/ut-issl/capsula/pull/876,> <https://github.com/ut-issl/capsula/pull/879_>)
- Update rust:1.94.1 Docker digest (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/870_>)
- Update postgres:18 Docker digest (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/862_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/860_>)
- Bump version to v0.12.0 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/880_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.11.5...v0.12.0>

## v0.11.5

> [!NOTE]
> v0.11.4 has been skipped. The crates.io publish workflow for v0.11.4 failed partway through, leaving only `capsula-api-types` published at that version. To ensure all crates are published consistently, the version was bumped to v0.11.5. There are no changes between v0.11.4 and v0.11.5.

### New Features

#### Add `tag_head` option to the `capture-git-repo` hook

A new `tag_head` config option (default `false`) creates a lightweight Git tag `capsula/<run-name>` at the HEAD commit when a run is captured. This prevents Git from garbage-collecting the commit after branch deletion or history rewriting (rebase, squash merge, etc.). The created tag name is included in the hook's JSON output as a `tag` field.

Closes #832.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/852_>

### Documentation

#### Add missing subcommands to CLI reference

Documents several previously undocumented CLI subcommands (`run-start`, `run-end`, `show`, `push`, `tui`, `vaults list`) and the `--vault-path` global option in the CLI reference.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/855_>

### Internal Changes

- Add cooldown period to Renovate config (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/850_>)
- Pin dependencies (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/851_>)
- Add trusted publishing workflow for crates.io (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/854_>)
- Pin GitHub Action versions (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/856_>)
- Update rust-lang/crates-io-auth-action action to v1.0.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/857_>)
- Bump version to v0.11.4 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/858_>)
- Bump version to v0.11.5 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/859_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.11.3...v0.11.5>

## v0.11.3

### New Features

#### Add `capsula show <run-name>` command

A new subcommand to display detailed information about a specific run, including metadata, command result, and hook summaries. Supports a `--json` flag for machine-readable output.

```bash
capsula show happy-river
capsula show happy-river --json
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/846_>

#### Add terminal UI (`capsula tui`)

A new `capsula tui` subcommand launches a mouse-clickable terminal interface (built with ratatui) for starting and ending runs. Designed for users who may not be comfortable with the command line.

- Start/end runs with visual feedback and an elapsed timer
- "Instant run" checkbox to execute both pre-run and post-run hooks in one go
- Mouse-clickable buttons and keyboard shortcuts (q, Tab, Enter/Space)
- Status indicators during hook execution and a success message on completion
- Quit confirmation dialog when a run is active

As part of this change, shared config/dotenv/vault-path loading logic was extracted into `capsula_orchestration::setup`, removing duplicated code from the CLI.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/847_>

### Bug Fixes

#### Fix capture-command hook to run in project root directory

The `capture-command` hook now correctly executes commands in the project root directory instead of the current working directory.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/840_>

### Internal Changes

- Update Rust crate toml to v1.1.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/834,> <https://github.com/ut-issl/capsula/pull/843,> <https://github.com/ut-issl/capsula/pull/845_>)
- Update Rust crate askama to v0.15.6 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/835_>)
- Update Rust crate sha2 to 0.11 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/836_>)
- Update actions/deploy-pages action to v5 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/837_>)
- Update actions/configure-pages action to v6 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/841_>)
- Update codecov/codecov-action action to v6 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/839_>)
- Update crate-ci/typos action to v1.45.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/844_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/833,> <https://github.com/ut-issl/capsula/pull/842_>)
- Bump version to v0.11.3 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/848_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.11.2...v0.11.3>

## v0.11.2

### New Features

#### Add `run-start` and `run-end` subcommands for manual run lifecycle

Two new subcommands support a manual run lifecycle where pre-run and post-run hooks are invoked separately, without a command execution in between. This is useful when the actual analysis runs independently (e.g., triggered by a GUI) and is not managed by Capsula.

- **`capsula run-start`**: Creates a new run directory, executes pre-run hooks, and prints the auto-generated run name to stdout.
- **`capsula run-end <run-name>`**: Finds an existing run directory by name, executes post-run hooks, and writes `post-run.json`. Guards against double-finalization.

Example usage:

```bash
# Before analysis
name=$(capsula run-start 2>/dev/null)

# ... external analysis runs independently ...

# After analysis
capsula run-end "$name"
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/819_>

#### Extract CLI logic into `capsula-orchestration` crate

Reusable orchestration logic has been extracted from `capsula-cli` into a new `capsula-orchestration` crate, making the CLI a thin shell. This enables future reuse from other contexts such as a PyO3 Python wrapper.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/830_>

### Security

- Update `aws-lc-rs` and `aws-lc-sys` to address CVE-2026-4428 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/829_>)
- Bump `rustls-webpki` from 0.103.9 to 0.103.10 (_by @dependabot in <https://github.com/ut-issl/capsula/pull/827_>)

### Internal Changes

- Update Rust crate tracing-subscriber to v0.3.23 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/822_>)
- Update Rust crate toml to v1.0.7 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/824_>)
- Update Rust crate askama_web to v0.15.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/825_>)
- Update Rust crate askama to v0.15.5 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/828_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/823_>)
- Bump version to v0.11.2 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/831_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.11.1...v0.11.2>

## v0.11.1

### Bug Fixes

#### Fix the command to install from Git

Fixed the install command documented for installing Capsula directly from the Git repository.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/792_>

### Security

- Update dependencies in lockfile to resolve a security issue in `aws-lc-sys` crate (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/810_>)
- Bump `quinn-proto` from 0.11.13 to 0.11.14 (_by @dependabot in <https://github.com/ut-issl/capsula/pull/816_>)

### Internal Changes

- Update Rust crate clap to v4.6.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/820_>)
- Update Rust crate clap to v4.5.60 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/799_>)
- Update Rust crate clap to v4.5.59 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/797_>)
- Update Rust crate toml to v1.0.6 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/811_>)
- Update Rust crate toml to v1.0.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/809_>)
- Update Rust crate toml to v1.0.3 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/798_>)
- Update Rust crate toml to v1.0.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/796_>)
- Update Rust crate tokio to v1.50.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/808_>)
- Update Rust crate anyhow to v1.0.102 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/800_>)
- Update Rust crate chrono to v0.4.44 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/803_>)
- Update Rust crate sysinfo to v0.38.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/815_>)
- Update Rust crate sysinfo to v0.38.3 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/807_>)
- Update Rust crate tempfile to v3.27.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/818_>)
- Update Rust crate tempfile to v3.26.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/804_>)
- Update Rust crate assert_cmd to v2.2.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/817_>)
- Update Rust crate testcontainers-modules to 0.15 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/801_>)
- Update crate-ci/typos action to v1.44.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/805_>)
- Update crate-ci/typos action to v1.43.5 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/795_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/794,> <https://github.com/ut-issl/capsula/pull/802,> <https://github.com/ut-issl/capsula/pull/806,> <https://github.com/ut-issl/capsula/pull/812_>)
- Bump version to v0.11.1 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/821_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.11.0...v0.11.1>

## v0.11.0

### Breaking Changes

#### CLI crate renamed from `capsula-cli` to `capsula`

The CLI crate has been renamed from `capsula-cli` to `capsula`. This affects users who depend on the crate by name (e.g., `cargo install capsula-cli`). The install command is now:

```bash
cargo install capsula
```

The `capsula-cli` workspace dependency entry in the root `Cargo.toml` has also been removed accordingly.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/790_>

### Internal Changes

- Update Rust crate anyhow to v1.0.101 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/776_>)
- Bump time from 0.3.46 to 0.3.47 (_by @dependabot in <https://github.com/ut-issl/capsula/pull/775_>)
- Update Rust crate sysinfo to v0.38.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/780_>)
- Update Rust crate reqwest to v0.13.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/779_>)
- Update Rust crate tempfile to v3.25.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/783_>)
- Update Rust crate toml to v1.0.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/787,> <https://github.com/ut-issl/capsula/pull/788_>)
- Update Rust crate clap to v4.5.58 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/785_>)
- Update Rust crate predicates to v3.1.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/786_>)
- Update crate-ci/typos action to v1.43.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/777,> <https://github.com/ut-issl/capsula/pull/778,> <https://github.com/ut-issl/capsula/pull/782_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/781_>)
- Bump version to v0.11.0 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/791_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.10.0...v0.11.0>

## v0.10.0

### Breaking Changes

#### MSRV bumped to 1.91

The minimum supported Rust version is now 1.91.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/748_>

### New Features

#### `require_pushed` option for `capture-git-repo` hook

Added `require_pushed` and `remote` config options to the `capture-git-repo` hook. When `require_pushed = true`, the hook verifies that the HEAD commit is reachable from the configured remote, aborting the run if it is not. This ensures that others can access the exact commit used for an experiment.

A new `is_pushed` boolean field is added to the hook's JSON output, always populated regardless of the `require_pushed` setting.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."
allow_dirty = false
require_pushed = true
remote = "origin"  # default
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/764_>

#### `run-dir` command

Added `capsula run-dir` subcommand to print the run directory for a given run name. When duplicate names exist, the newest run is preferred.

```bash
capsula run-dir jaded-cat
# /path/to/.capsula/my-vault/2026-01-30/123456-jaded-cat
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/771_>

#### Run Query API with JSONPath filters (server)

Added a new `POST /api/v1/runs/search` endpoint that enables querying runs by hook output values using SQL/JSON path expressions. This allows consumers to find runs matching specific criteria (e.g., git commit, parameter values, file hashes) without needing a Capsula workspace.

```json
{
  "vault": "my-project",
  "hook_filters": [
    {
      "hook_id": "capture-git-repo",
      "output_filter": "$.sha ? (@ starts with \"abc123\")"
    }
  ],
  "order": "latest_first",
  "limit": 10
}
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/758_>

#### Environment variables from dotenv files for vault path and server URL

Fixed the timing issue where environment variables specified in dotenv files were not being used for `vault.path` and `server` configuration options. Added `--vault-path` global CLI option and manual environment variable resolution after dotenv loading.

Priority order: CLI argument > environment variable (after dotenv) > config file.

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/751_>

### Bug Fixes

#### Log messages printed to stderr

Log messages from `tracing_subscriber` are now printed to stderr instead of stdout, following the standard Unix convention. This fixes issues where log output was mixed with program data (e.g., `capsula run-dir` output).

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/772_>

### Documentation

- Document `name` field for `capture-git-repo` hook (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/761_>)

### Internal Changes

- Make some Clippy config explicit in Cargo.toml (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/749_>)
- Use stable Rust version in dev, check with MSRV in CI (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/757_>)
- Bump bytes from 1.11.0 to 1.11.1 (_by @dependabot in <https://github.com/ut-issl/capsula/pull/770_>)
- Update Rust crate clap to v4.5.57 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/769_>)
- Update Rust crate clap to v4.5.56 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/762_>)
- Update Rust crate clap to v4.5.55 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/756_>)
- Update Rust crate git2 to v0.20.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/766_>)
- Update Rust crate sysinfo to 0.38 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/750_>)
- Update Rust crate askama to v0.15.4 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/759_>)
- Update Rust crate askama to v0.15.3 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/754_>)
- Update Rust crate askama_web to v0.15.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/747_>)
- Update crate-ci/typos action to v1.43.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/768_>)
- Update crate-ci/typos action to v1.43.0 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/767_>)
- Update crate-ci/typos action to v1.42.3 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/755_>)
- Update crate-ci/typos action to v1.42.2 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/753_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/765,> <https://github.com/ut-issl/capsula/pull/752_>)
- Bump version to v0.10.0 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/774_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.9.5...v0.10.0>

## v0.9.5

### New Features

#### Configurable upload body size limit for capsula-server

Added `--max-body-size` CLI option and `CAPSULA_MAX_BODY_SIZE` environment variable to configure the maximum upload body size for the server. The default is 100MB (104,857,600 bytes).

This resolves an issue where file uploads larger than 2MB were failing due to Axum's default body size limit.

**Usage:**

```bash
# CLI
capsula-server --max-body-size 209715200  # 200MB

# Environment variable
export CAPSULA_MAX_BODY_SIZE="209715200"
capsula-server
```

_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/744_>

### Internal Changes

- Add capture-file post-run hook to repository config (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/745_>)
- Update crate-ci/typos action to v1.42.1 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/741_>)
- Update Rust crate chrono to v0.4.43 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/738_>)
- Update Rust crate thiserror to v2.0.18 (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/739_>)
- Lock file maintenance (_by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/735,> <https://github.com/ut-issl/capsula/pull/740_>)
- Bump version to 0.9.5 (_by @shunichironomura in <https://github.com/ut-issl/capsula/pull/746_>)

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.9.4...v0.9.5>

## v0.9.4

### New Features

#### Capsula Server

This releases adds a Capsula server. Users can push the local run data to it, and it displays the runs in a Web UI.

Related PRs:

- Add Capsula server by @shunichironomura in <https://github.com/ut-issl/capsula/pull/719>
- capsula-server-cli by @shunichironomura in <https://github.com/ut-issl/capsula/pull/729>
- Containerize Capsula server by @shunichironomura in <https://github.com/ut-issl/capsula/pull/730>
- Use latest Rust version in container by @shunichironomura in <https://github.com/ut-issl/capsula/pull/732>
- Use port 8500 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/733>

#### Behavior of `capsula --version`

When Capsula is installed from GitHub, `capsula --version` displays the Git commit SHA in addition to its version.

Related PRs:

- Add commit hash to the output of `capsula --version` by @shunichironomura in <https://github.com/ut-issl/capsula/pull/716>
- Hide (commit: unknown) if Git commit is unknown by @shunichironomura in <https://github.com/ut-issl/capsula/pull/717>

### Internal

- Change deny rules to warn and allow multiple_crate_versions by @shunichironomura in <https://github.com/ut-issl/capsula/pull/711>
- Add coverage report and refactor code by @shunichironomura in <https://github.com/ut-issl/capsula/pull/725>
- Make the next version bump a patch, not a minor by @shunichironomura in <https://github.com/ut-issl/capsula/pull/734>

### Documentation

- Setup preliminary docs by @shunichironomura in <https://github.com/ut-issl/capsula/pull/707>
- Refine docs by @shunichironomura in <https://github.com/ut-issl/capsula/pull/726>

### Dependencies

- Update actions/checkout action to v6 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/708>
- Update actions/setup-python action to v6 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/709>
- Update Rust crate reqwest to 0.13 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/710>
- Update crate-ci/typos action to v1.41.0 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/712>
- Update Rust crate clap to v4.5.54 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/713>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/714>
- Update Rust crate serde_json to v1.0.149 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/715>
- Update crate-ci/typos action to v1.42.0 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/718>
- Update postgres Docker tag to v18 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/723>
- Update Rust crate testcontainers-modules to 0.14 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/722>
- Update Askama to v0.15 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/724>
- Update Rust crate toml to v0.9.11 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/728>
- Update Rust crate assert_cmd to v2.1.2 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/727>

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.9.3...v0.9.4>

## v0.9.3

### New Features

- Support Slack attachment in `notify-slack` hook by @shunichironomura in <https://github.com/ut-issl/capsula/pull/702>
- Add `dotenv` config to load environment variables from a dotenv file by @shunichironomura in <https://github.com/ut-issl/capsula/pull/705>

### Documentation

- Fix Slack bot set-up instructions by @shunichironomura in <https://github.com/ut-issl/capsula/pull/704>

### Internal

- Set up dev tools and improve logging by @shunichironomura in <https://github.com/ut-issl/capsula/pull/701>
- Add debug logging to hooks by @shunichironomura in <https://github.com/ut-issl/capsula/pull/703>
- Bump version to 0.9.3 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/706>

### Dependency Updates

- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/700>

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.9.2...v0.9.3>

## v0.9.2

### New Features

- Support reading Slack bot token from `SLACK_BOT_TOKEN` environment variable by @shunichironomura in <https://github.com/ut-issl/capsula/pull/697>

### Bug Fixes

- Fix handling of glob in subdirectories in `capture-file` hook by @shunichironomura in <https://github.com/ut-issl/capsula/pull/688>

### Documentation

- Update README with a section on `notify-slack` hook by @shunichironomura in <https://github.com/ut-issl/capsula/pull/698>

### Internal

- Make clippy more strict by @shunichironomura in <https://github.com/ut-issl/capsula/pull/679>
- Change the type of `total_memory` field of `MachineCaptured` struct from usize to u64 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/680>
- Enable Clippy nursery lints and some of the restricted lints by @shunichironomura in <https://github.com/ut-issl/capsula/pull/681>
- Move glob crate to workspace dependencies by @shunichironomura in <https://github.com/ut-issl/capsula/pull/695>
- Add tracing by @shunichironomura in <https://github.com/ut-issl/capsula/pull/694>
- Bump MSRV to 1.90 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/696>
- Bump version to 0.9.2 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/699>

### Dependency Updates

- Update Rust crate git2 to v0.20.3 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/677>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/678>
- Update Rust crate reqwest to v0.12.25 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/682>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/684>
- Update Rust crate reqwest to v0.12.26 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/685>
- Update Rust crate toml to v0.9.9 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/686>
- Update Rust crate toml to v0.9.10 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/687>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/689>
- Update Rust crate serde_json to v1.0.146 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/690>
- Update Rust crate reqwest to v0.12.28 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/691>
- Update Rust crate serde_json to v1.0.147 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/692>
- Update Rust crate serde_json to v1.0.148 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/693>

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.9.1...v0.9.2>

## v0.9.1

### Bug Fixes

- Fixed a bug where `capture-git-repo` reports that the repository is dirty even if it is not by @shunichironomura in <https://github.com/ut-issl/capsula/pull/673>

### Other Changes

- Remove `--branch rust` option from installation command from GitHub by @shunichironomura in <https://github.com/ut-issl/capsula/pull/656>
- Update `CLAUDE.md` and `README.md` by @shunichironomura in <https://github.com/ut-issl/capsula/pull/657>
- `justfile` minor updates by @shunichironomura in <https://github.com/ut-issl/capsula/pull/658>
- Add license files by @shunichironomura in <https://github.com/ut-issl/capsula/pull/659>
- Change the `toml` dependency spec and move it to the workspace dependency by @shunichironomura in <https://github.com/ut-issl/capsula/pull/660>
- Add a section on license to README by @shunichironomura in <https://github.com/ut-issl/capsula/pull/661>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/662>
- Fix error message on absent config file by @shunichironomura in <https://github.com/ut-issl/capsula/pull/665>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/666>
- Update Rust crate clap to v4.5.52 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/668>
- Update Rust crate clap to v4.5.53 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/669>
- Update actions/checkout action to v6 by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/670>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/671>
- Lock file maintenance by @renovate[bot] in <https://github.com/ut-issl/capsula/pull/672>
- Set allow_dirty to false in capture-git-repo hook by @shunichironomura in <https://github.com/ut-issl/capsula/pull/674>
- Bump version to 0.9.1 by @shunichironomura in <https://github.com/ut-issl/capsula/pull/675>

**Full Changelog**: <https://github.com/ut-issl/capsula/compare/v0.9.0...v0.9.1>

## v0.9.0

> [!IMPORTANT]
> From this release, the Git tag has no longer has prefix `rs-`. Note that the tags without the `rs-` prefix prior to `v0.9.0` is the Python variant of Capsula that is no longer maintained.

### Highlights

#### Export `CAPSULA_PRE_RUN_OUTPUT_PATH` by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/640>

You can now access the output JSON file path of the pre-run hooks in the command execution context by accessing the `CAPSULA_PRE_RUN_OUTPUT_PATH` environment variable.

#### Add project root to run metadata by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/647>

The output `metadata.json` now has `project_root` information.

#### Fix `HookFailed` error by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/649>

The hook names inside the `error` field in case of hook run failure has been changed.

#### Minimal Slack notification hook by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/650>

A very minimal Slack notification hook has been implemented, and you can use it as a pre-run hook and a post-run hook.
Use it with the following configuration:

```toml
# As a pre-run hook
[[pre-run.hooks]]
id = "notify-slack"
token = "xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxxxxxxxxxxxxxx"
channel = "#general"

# As a post-run hook
[[post-run.hooks]]
id = "notify-slack"
token = "xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxxxxxxxxxxxxxx"
channel = "#general"
```

### Documentation

- Add section on available environment variables in README by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/641>
- Update README on the name of the Python branch by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/643>

### Internal

- Run CI in every PR by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/642>
- Simplify `Hook` and `HookErased` methods by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/645>
- Make hooks generic over phase by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/646>
- Remove `phase` from `RuntimeParams` by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/648>
- Simplify JSON serialization of `Captured` by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/651>
- Add tests by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/652>
- Remove features from `capsula-registry` crate by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/653>
- Bump to v0.9.0 by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/654>
- Fix crate info and justfile by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/655>

**Full Changelog**: <https://github.com/shunichironomura/capsula/compare/rs-v0.3.0...v0.9.0>

## rs-v0.3.0

### Highlights

#### Contexts have been renamed to hooks (breaking change)

In this release, we have renamed "contexts" to "hooks" for better future extensibility. Each hook now has an `id` field instead of `type`, following the pre-commit hook convention. The configuration and output format have also been updated accordingly. Please refer to the updated documentation for details on the new configuration format.

- Rename context `type` to `id` by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/614>
- Rename contexts by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/616>
- Rename context to hook by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/618>
- Remove watchers by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/620>
- Restructure config by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/621>
- Remove ID from run directory by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/622>
- Remove redundant config by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/627>
- Improve output by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/630>

#### `capsula list` command

`capsula list` command has been implemented to list all captured runs.

- Implement minimal `capsula list` command by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/631>
- Remove `ID` column and make `COMMAND` column wider by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/632>

#### `capsula-capture-git-repo`

`capsula-capture-git-repo` (formerly `capsula-git-context`) hook now captures the diffs of the current Git repository and saves them as patch files.

- Save a patch file if the repository is dirty by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/609>

### Documentation

- Add a note on Python Capsula by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/584>
- Update README by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/623>
- Update README by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/633>

### Internal

- resolve-clippy-errors by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/585>
- add pre-commit config, renovate config, and CI by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/586>
- Add cargo check in CI by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/589>
- [pre-commit.ci] pre-commit autoupdate by @pre-commit-ci[bot] in <https://github.com/shunichironomura/capsula/pull/595>
- Set `default-features` of `names` to `false` by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/607>
- add `just lint` command by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/608>
- Simplify error handling by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/626>
- Add default to justfile by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/628>
- Add tests to CI by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/629>
- Bump version to 0.3.0 by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/634>
- Fix `just publish` command by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/635>

### Dependency Updates

- Update Rust crate anyhow to v1.0.100 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/587>
- Update Rust crate clap to v4.5.48 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/588>
- Lock file maintenance by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/594>
- Update actions/checkout action to v5 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/593>
- Update Rust crate serde to v1.0.228 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/601>
- Update Rust crate toml to v0.9.8 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/604>
- Update Rust crate thiserror to v2.0.17 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/603>
- Update Rust crate sysinfo to v0.37.2 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/602>
- Update Rust crate clap to v4.5.49 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/606>
- Lock file maintenance by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/605>
- Lock file maintenance by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/610>
- Update Rust crate clap to v4.5.50 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/613>
- Lock file maintenance by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/624>
- Update Rust crate clap to v4.5.51 by @renovate[bot] in <https://github.com/shunichironomura/capsula/pull/625>

**Full Changelog**: <https://github.com/shunichironomura/capsula/compare/rs-v0.2.0...rs-v0.3.0>

## rs-v0.2.0

### What's Changed

- Add a context to capture environment variables by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/576>
- Command context by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/577>
- Add machine context by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/578>
- Improve error handling by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/579>
- readme by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/580>
- Bump version to 0.2.0 by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/581>
- Add descriptions by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/582>
- Comment-out unused config struct by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/583>

**Full Changelog**: <https://github.com/shunichironomura/capsula/compare/rs-v0.1.0...rs-v0.2.0>

## rs-v0.1.0

Rust Capsula Initial Release

This is the initial release of Capsula redesigned and implemented in Rust.
The development is active on the [`rust` branch](https://github.com/shunichironomura/capsula/tree/rust).
Use with caution as the implementation and documentation are not yet matured.

Note that it shares a lot of concepts such as contexts with the Python Capsula, the API design has been significantly changed.
Notably, users will use the Rust Capsula via CLI: `capsula` command.

### Installation

#### Install from crates.io

```bash
cargo install capsula-cli --locked
```

#### Install from the GitHub repository

```bash
cargo install --git https://github.com/shunichironomura/capsula --branch rust --locked capsula-cli
```

### What's Changed

- cwd context by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/552>
- Move crates to `crates` directory by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/555>
- Update workspace config by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/556>
- CwdContext capture by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/557>
- git-context by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/558>
- config crate by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/559>
- Include context type in output by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/560>
- markdownlint by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/562>
- Improve context registry by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/561>
- `capsula run` command by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/563>
- Improve context type handling by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/564>
- file-context by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/565>
- Add installation instructions to README by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/566>
- Fix tests by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/567>
- Prepare for crates.io release by @shunichironomura in <https://github.com/shunichironomura/capsula/pull/568>

**Full Changelog**: <https://github.com/shunichironomura/capsula/commits/rs-v0.1.0>
