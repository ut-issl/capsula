# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Capsula is a CLI tool for capturing and preserving the context of command executions. It records the state of a project environment before and after running commands for reproducibility and auditing. The project is written in Rust and organized as a workspace with multiple crates.

**Important:** The Python version of Capsula is deprecated and found on the main branch. The current Rust implementation is the active development branch.

## Development Commands

### Building

```bash
# Build the entire workspace
cargo build --workspace

# Build only the CLI
cargo build -p capsula

# Build for release
cargo build --release
```

### Testing

```bash
# Run all tests in the workspace
cargo test --workspace

# Run tests for a specific crate
cargo test -p capsula-core

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Linting

```bash
# Run all lints (using justfile)
just lint

# Individual lint commands (if needed):
cargo clippy --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --no-default-features
cargo fmt --check --all
cargo doc --workspace --no-deps
cargo check --workspace
```

### Running

```bash
# Install locally from source
cargo install --path crates/capsula-cli --locked capsula

# Run directly with cargo
cargo run -p capsula -- run echo "hello"
cargo run -p capsula -- list

# Run with a specific config
cargo run -p capsula -- --config path/to/config.toml run python script.py
```

## Architecture

### Crate Structure (3-Tier Hierarchy)

The workspace consists of 11 crates organized into three dependency tiers:

**Tier 1 - Core Infrastructure:**

- **capsula-core**: Foundation traits and types (`Hook<P>`, `Captured`, `HookErased<P>`, `Run<Dir>`, `CapsulaError`)

**Tier 2 - System Support:**

- **capsula-registry**: Hook type registry and factory pattern (maps hook IDs to creation functions)
- **capsula-config**: TOML configuration parsing into strongly-typed structs

**Tier 3 - CLI and Hook Implementations:**

- **capsula**: Command-line interface and orchestration (main entry point)
- **Hook implementation crates** (7 total):
  - `capsula-capture-cwd`: Current working directory
  - `capsula-capture-env`: Environment variables
  - `capsula-capture-git-repo`: Git repository state (commit hash, dirty status)
  - `capsula-capture-file`: File content capture/hashing
  - `capsula-capture-machine`: System information (CPU, memory, OS)
  - `capsula-capture-command`: Shell command output
  - `capsula-notify-slack`: Slack notifications

### Core Trait System

**The `Hook<P>` trait** (`crates/capsula-core/src/hook.rs`): Generic, type-safe interface for all hooks

- Generic parameter `P: PhaseMarker` distinguishes `PreRun` vs `PostRun` phases at compile time
- Each hook has a unique string ID, strongly-typed Config struct, and Output struct

**The `HookErased<P>` trait**: Object-safe trait for heterogeneous hook collections

- Enables storing different hook types in `Vec<Box<dyn HookErased<P>>>`
- Blanket impl converts all `Hook<P>` to `HookErased<P>` automatically

**The `Captured` trait** (`crates/capsula-core/src/captured.rs`): Output contract for all hook results

- Must be JSON-serializable
- Can optionally request run abortion (e.g., git hook when dirty and `allow_dirty=false`)

### Key Design Patterns

**Factory + Registry Pattern**: The registry stores function pointers (not type information), enabling dynamic hook creation by string ID from configuration. Each hook type provides a `from_config` function that deserializes JSON config and returns `Box<dyn HookErased<P>>`.

**Type State Pattern**: The `Run<Dir>` struct uses phantom types to enforce setup ordering at compile time:

- `Run<()>`: Unprepared run, directory not yet created (cannot execute)
- `Run<PathBuf>`: Prepared run, directory exists (can execute)
- `.setup_run_dir()` transforms `Run<()>` → `Run<PathBuf>`

**Phase-based Execution**: Hooks are organized into two phases using phantom types (`PreRun` and `PostRun`):

- **Pre-run phase**: Captured before command execution (e.g., git state, input files)
- **Post-run phase**: Captured after command execution (e.g., output files, final state)

**JSON as Configuration Interchange**: Hook configs are stored as `serde_json::Value` (dynamic JSON) rather than statically typed. This enables different config types per hook without recompiling the config parser.

### Configuration to Execution Pipeline

```
capsula.toml
    ↓ Parse TOML
CapsulaConfig { vault, pre_run: [HookEnvelope], post_run: [HookEnvelope] }
    ↓ For each HookEnvelope
registry.create_hook(id, config_json, project_root)
    ↓ Lookup factory function by ID
Hook::from_config() → deserialize JSON to typed Config
    ↓ Create instance
Box<dyn HookErased<P>>
    ↓ Collect all hooks
Vec<Box<dyn HookErased<P>>>
    ↓ Execute each hook
hook.run(metadata, params) → Box<dyn Captured>
    ↓ Serialize with metadata
JSON output with __meta field
```

### Execution Flow

1. CLI parses arguments and loads `capsula.toml`
2. Create pre-run and post-run registries with `standard_pre_run_hook_registry()` and `standard_post_run_hook_registry()`
3. Build `Run<()>` with generated ULID and random name (e.g., "chubby-back")
4. Setup run directory: `Run<()>` → `Run<PathBuf>` (creates `.capsula/{vault-name}/{YYYY-MM-DD}/{HHMMSS-name}/_capsula/`)
5. Write `metadata.json`
6. **Pre-run phase**: Build hooks from config, execute all in order, serialize to `pre-run.json`, check `abort_requested()` flags
7. **Command execution**: Spawn child process with environment variables set, capture stdout/stderr in parallel threads
8. **Post-run phase**: Build and execute post-run hooks, serialize to `post-run.json`

### Output Structure

```
.capsula/{vault-name}/{YYYY-MM-DD}/{HHMMSS-name}/
├── _capsula/              # Capsula metadata directory
│   ├── metadata.json      # Run metadata (ID, name, command, timestamp)
│   ├── pre-run.json       # Array of pre-phase hook outputs
│   ├── command.json       # Command execution results (exit_code, stdout, stderr, duration)
│   └── post-run.json      # Array of post-phase hook outputs
└── [captured files]       # Files copied by file hooks
```

Each hook's JSON output includes a `__meta` field with `id`, `config`, and `success` status. Failed hooks have `success: false` and an `error` field, but don't stop other hooks from running.

### Environment Variables

When executing a command with `capsula run`, these environment variables are set:

- `CAPSULA_RUN_ID`: ULID of the run
- `CAPSULA_RUN_NAME`: Human-readable generated name
- `CAPSULA_RUN_DIRECTORY`: Absolute path to the run directory
- `CAPSULA_RUN_TIMESTAMP`: ISO 8601 timestamp
- `CAPSULA_RUN_COMMAND`: Shell-quoted command string
- `CAPSULA_PRE_RUN_OUTPUT_PATH`: Path to pre-run.json
- `CAPSULA_PROJECT_ROOT`: Project root directory

### Error Handling Strategy

**Non-fatal hook errors**: Each hook's error is caught, logged as a warning, and stored in `__meta.error` field. Execution continues with remaining hooks.

**Fatal errors** (abort execution):

- Configuration parse errors
- Run directory creation failures
- Command execution failures
- Hook requests abort via `Captured::abort_requested()` (e.g., dirty git repo when `allow_dirty=false`)

This design ensures partial success is always recorded, valuable for debugging.

### Hook Implementation Pattern

Each hook crate follows this structure:

1. **Config struct** (deserializable): `#[derive(Deserialize, Serialize)] pub struct XHookConfig { ... }`
2. **Captured output struct** (serializable): `#[derive(Serialize)] pub struct XCaptured { ... }`
3. **Hook struct** (implements `Hook<P>`): Contains config and implements trait with `ID`, `Config`, `Output` types
4. **Captured implementation**: Implements `serialize_json()` and optionally `abort_requested()`

Example file locations:

- CwdHook (simplest): `crates/capsula-capture-cwd/src/lib.rs`
- GitHook (with abort logic): `crates/capsula-capture-git-repo/src/lib.rs`
- FileHook (complex config): `crates/capsula-capture-file/src/lib.rs`

## Adding a New Hook Type

1. Create new crate `capsula-{hook-id}` in `crates/`
2. Implement `Hook<P>` trait (works for both PreRun and PostRun via generic)
3. Implement `Captured` trait for output type
4. Add dependency in `capsula-registry/Cargo.toml`
5. Register in `capsula-registry/src/lib.rs` using `.with_hook::<YourHook>()` in both registries
6. No changes needed to CLI or config parser
