# AGENTS.md

This file contains repository guidance for coding agents. `CLAUDE.md` is a symlink to this file.

## Project Overview

Capsula is a Rust command-line tool that captures the context of command executions for reproducibility, traceability, auditing, and debugging. It records pre-run state, runs a command while streaming and capturing stdout/stderr, records post-run state, and can push completed runs to a PostgreSQL-backed server.

## Repository Layout

The workspace contains crates under `crates/`:

- `capsula` (`capsula-cli`): command-line application and command dispatch.
- `capsula-core`: core errors, captured output, phase-aware hooks, and the type-state run model.
- `capsula-config`: TOML configuration parsing and hook envelopes.
- `capsula-registry`: registry/factory for creating hooks from string IDs.
- `capsula-orchestration`: shared configuration loading, run lifecycle, hook execution, vault operations, and server push logic.
- `capsula-tui`: interactive terminal UI for manual runs.
- `capsula-server`: Axum/PostgreSQL web server, REST API, HTML UI, migrations, and file storage.
- `capsula-client`: blocking HTTP client used by the CLI to communicate with the server.
- `capsula-api-types`: shared serializable client/server API types.
- Hook crates: `capsula-capture-cwd`, `capsula-capture-env`, `capsula-capture-git-repo`, `capsula-capture-file`, `capsula-capture-machine`, `capsula-capture-command`, `capsula-capture-json`, `capsula-capture-toml`, and `capsula-notify-slack`.

The workspace has `crates/*` as members and `crates/capsula-cli` as its default member. Root-level `README.md` documents the basic CLI workflow; server-specific setup is in `crates/capsula-server/README.md`; hook documentation is in `docs/hooks/`.

## Development Commands

```bash
# Build all crates
cargo build --workspace

# Build or run the CLI
cargo build -p capsula
cargo run -p capsula -- --help
cargo run -p capsula -- run echo "hello"
cargo run -p capsula -- list

# Run all tests or one crate/test
cargo test --workspace
cargo test -p capsula-core
cargo test test_name
cargo test -- --nocapture

# Run the repository lint/check suite
just lint
# Equivalent individual checks:
cargo clippy --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --no-default-features
cargo fmt --check --all
cargo doc --workspace --no-deps
cargo check --workspace

# Coverage
just coverage
just coverage-lcov
```

CI additionally checks SQLx prepared queries, the declared MSRV, and spelling. Keep `crates/capsula-server/.sqlx/` synchronized when changing SQLx queries:

```bash
just start-db
# or start PostgreSQL with docker compose, then:
just sqlx-prepare
```

`just start-db`, `just stop-db`, `just serve`, and the SQLx recipe use `.env.server` and require `dotenvx` plus Docker/PostgreSQL as appropriate. `just serve` runs the server on its default port.

## CLI Commands and Configuration

The CLI reads `capsula.toml` by default; use global `--config <path>` for another file. The available commands are:

- `run <command> [args...]`: create a run, execute pre-run hooks, run the command, then execute post-run hooks.
- `run-start`: create a run and execute only pre-run hooks; prints the generated run name.
- `run-end <run-name>`: execute post-run hooks for a run created by `run-start`.
- `run-dir <run-name>`: print a run directory.
- `list`: list local runs.
- `show <run-name>` and `show --json <run-name>`: inspect a local run.
- `push [run-id-or-name]` or `push --all`: upload runs to the configured server.
- `vaults list`: list server-side vaults.
- `tui`: launch the interactive terminal UI for manual runs.

Global `--vault-path <path>` overrides the configured vault location. Relative paths are resolved from the project root. Server options accept `--server <URL>` and otherwise use `CAPSULA_SERVER_URL`, then the top-level `server` value in `capsula.toml`.

A minimal configuration is:

```toml
[vault]
name = "my-project"
# path = ".capsula/my-project" # optional; this is the default

[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."
allow_dirty = false
```

Supported top-level configuration fields are:

- `[vault]`: required `name`, optional `path`.
- `dotenv`: optional dotenv file path, resolved relative to the project root.
- `server`: optional server URL.
- `[[pre-run.hooks]]` and `[[post-run.hooks]]`: ordered hook entries with an `id` and hook-specific fields.

The standard registries currently provide these hook IDs in both phases:

- `capture-cwd`: capture the current working directory.
- `capture-env`: capture a named environment variable.
- `capture-git-repo`: capture commit, dirty, pushed state, and optionally a diff/tag; can request an abort when dirty or not pushed.
- `capture-file`: glob files, optionally copy/move them into the run artifact directory, and/or hash them with SHA-256.
- `capture-machine`: capture host/system information.
- `capture-command`: execute a command and capture output, exit status, and duration; `abort_on_failure` is available.
- `capture-json`: parse one JSON file into queryable `content`.
- `capture-toml`: parse one TOML file into queryable JSON `content`.
- `notify-slack`: send Slack notifications; see `docs/hooks/notify-slack.md` for token and attachment configuration.

Each hook configuration is deserialized by its hook crate. Hook configs generally use `#[serde(deny_unknown_fields)]`, so check the relevant documentation before adding fields.

## Architecture and Execution Flow

The core hook API is in `crates/capsula-core/src/hook.rs`:

- `Hook<P>` is a typed hook trait where `P` is `PreRun` or `PostRun`.
- `HookErased<P>` is the object-safe trait used for heterogeneous hook collections.
- `PhaseMarker::phase_name()` supplies `pre`/`post` artifact directory names.
- `RuntimeParams<P>` optionally carries a per-hook artifact directory.
- `Captured` serializes hook output and can request abortion through `abort_requested()`.

`Run<Dir>` in `crates/capsula-core/src/run.rs` uses type state:

- `Run<()>` (`UnpreparedRun`) has metadata but no run directory and cannot execute.
- `setup_run_dir()` transforms it into `Run<PathBuf>` (`PreparedRun`), creates the vault/run directory, and retries collisions.
- `PreparedRun::exec()` runs the command directly (not through a shell), streams stdout/stderr to the console while capturing them, and returns exit code, output, and duration.

The normal `capsula run` flow is:

1. Parse the CLI and load `capsula.toml`; determine project root, dotenv variables, and vault path.
2. Create a ULID and human-readable random name, create the run directory, and write `_capsula/metadata.json`.
3. Build the pre-run hook list through the registry and execute hooks in configuration order.
4. Write `_capsula/pre-run.json`; if any captured result requests an abort, stop before the command with exit code `125`.
5. Execute the command and write `_capsula/command.json`.
6. Execute post-run hooks and write `_capsula/post-run.json`.

Hook execution is best-effort per hook: a hook failure is recorded in the result with `success: false` and an error, while later hooks still run. Hook configuration/build failures prevent the phase from starting. Successful outputs receive a `__meta` object containing the hook ID, serialized config, and success status.

Hooks that return `needs_artifact_dir() == true` receive a dedicated directory named `{phase}-{index}-{hook-id}/` under the run directory. The built-in file and Git hooks use this mechanism. The `_capsula/` directory contains metadata and JSON summaries; captured artifacts live beside it.

A completed local run looks like:

```text
.capsula/{vault-path}/{YYYY-MM-DD}/{HHMMSS-name}/
├── _capsula/
│   ├── metadata.json
│   ├── pre-run.json
│   ├── command.json
│   └── post-run.json
├── pre-0-capture-git-repo/
└── post-0-capture-file/
    └── captured-artifact
```

The command receives these environment variables: `CAPSULA_RUN_ID`, `CAPSULA_RUN_NAME`, `CAPSULA_RUN_DIRECTORY`, `CAPSULA_RUN_TIMESTAMP`, `CAPSULA_RUN_COMMAND`, `CAPSULA_PRE_RUN_OUTPUT_PATH`, and `CAPSULA_PROJECT_ROOT`.

## Server and Push Architecture

`capsula-server` is an Axum web server backed by PostgreSQL and SQLx. It runs embedded migrations on startup and stores deduplicated uploaded files under the configured storage path. The HTML UI is served at `/`, `/vaults`, `/runs`, and `/runs/{id}`.

Important API routes include:

- `GET /health`
- `GET /api/v1/vaults` and `GET /api/v1/vaults/{name}`
- `POST /api/v1/runs`, `GET /api/v1/runs`, `POST /api/v1/runs/search`, and `GET /api/v1/runs/{id}`
- `GET /api/v1/runs/{id}/files/{path}`
- `POST /api/v1/upload`

Run uploads include run metadata, command output, hook outputs, and captured files. `capsula push` posts metadata first, then uploads artifacts and pre/post hook results; `--all` walks every local run in the vault. The server uses `DATABASE_URL` by default and listens on `127.0.0.1:8500`. Other server settings are documented in `crates/capsula-server/README.md` and include `CAPSULA_HOST`, `CAPSULA_PORT`, `STORAGE_PATH`, `CAPSULA_MAX_CONNECTIONS`, `CAPSULA_MAX_BODY_SIZE`, and `RUST_LOG`.

## Adding a Hook

1. Add a crate under `crates/` following the existing hook crate pattern.
2. Define a serializable config and captured output type.
3. Implement `Hook<PreRun>` and/or `Hook<PostRun>`; implement both when the hook is valid in both phases.
4. Implement `Captured`; use `abort_requested()` only when the hook intentionally prevents command execution.
5. Add the crate to `[workspace.dependencies]` in the root `Cargo.toml` and to `crates/capsula-registry/Cargo.toml`.
6. Register the hook in `standard_hook_registry()` in `crates/capsula-registry/src/lib.rs`.
7. Add hook documentation under `docs/hooks/` and tests for configuration and execution behavior.

The registry is compile-time wired through `RegistryBuilder::with_hook::<YourHook>()`; no CLI or config-parser changes are needed for a normal hook.

## GitHub Issues and Pull Requests

When opening or updating a GitHub Pull Request, comply with `.github/pull_request_template.md`.

When an agent files a GitHub Issue or opens a GitHub Pull Request, put this alert note at the very top of the description. Add the same note at the beginning of any issue or pull request comment written by an agent:

> [!WARNING]
> This content was written by an AI agent and must be verified by a human developer. After human verification, this alert may be removed.

The human developer must remove this note after verifying the description or comment's contents.

## Error Handling and Code Style

- Use `Result` for fallible operations; do not use `None` as an error signal.
- Hook execution errors should normally be captured as non-fatal per-hook results by the orchestration layer.
- Configuration, run-directory setup, command execution, and server/database failures are fatal to the relevant operation.
- Preserve error context with `anyhow::Context` at application/orchestration boundaries and use `thiserror` for crate-specific errors.
- Follow the workspace Clippy configuration in the root `Cargo.toml`; CI runs with `RUSTFLAGS="-Dwarnings"`.
- Avoid editing generated directories such as `target/`, `.capsula/` run data, `storage/`, or SQLx cache files except through the appropriate command/workflow.
