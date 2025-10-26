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
cargo build -p capsula-cli

# Build with all features
cargo build --workspace --all-features

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

# Individual lint commands:
cargo clippy --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --no-default-features
cargo fmt --check --all
cargo doc --workspace --no-deps
cargo check --workspace
```

### Running

```bash
# Install locally from source
cargo install --path crates/capsula-cli --locked

# Run directly with cargo
cargo run -p capsula-cli -- run echo "hello"
cargo run -p capsula-cli -- capture --phase pre

# Run with a specific config
cargo run -p capsula-cli -- --config path/to/config.toml run python script.py
```

## Architecture

### Crate Structure

The project is a Cargo workspace containing these crates:

- **capsula-core**: Core abstractions and traits
  - `Context` trait: Interface for context capture implementations
  - `ContextErased`: Object-safe trait for heterogeneous context collections
  - `ContextFactory`: Factory trait for creating contexts from configuration
  - `Captured` trait: Interface for captured data output
  - `Run` struct: Manages command execution with context capture

- **capsula-registry**: Context type registration system
  - `ContextRegistry`: Maps context type names (e.g., "git", "file") to factories
  - `standard_registry()`: Creates registry with all built-in context types
  - Feature-gated registration: Only enabled context types are included

- **capsula-config**: Configuration parsing (TOML)
  - Parses `capsula.toml` configuration files
  - `CapsulaConfig`: Top-level configuration structure
  - `PhaseConfig`: Pre/post/in-phase configuration
  - `ContextEnvelope`: Type-erased context config with dynamic JSON fields

- **capsula-cli**: Command-line interface
  - Main entry point
  - Two commands: `capture` (standalone context capture) and `run` (execute with context)
  - Orchestrates configuration loading, registry setup, and context execution

- **Context implementation crates** (each implements a specific context type):
  - `capsula-git-context`: Git repository state (commit hash, dirty status)
  - `capsula-file-context`: File capture (copy/move files, compute hashes)
  - `capsula-cwd-context`: Current working directory
  - `capsula-env-context`: Environment variables
  - `capsula-command-context`: Shell command output
  - `capsula-machine-context`: System information (CPU, memory, OS)

### Key Design Patterns

**Factory + Registry Pattern**: Each context type provides a factory that knows how to parse its configuration and create instances. The registry maps type names to factories, enabling dynamic context creation from configuration.

**Trait Object Pattern**: The `ContextErased` trait enables storing heterogeneous context types in a single collection while maintaining type safety for the concrete implementations.

**Type State Pattern**: The `Run<Dir>` struct uses phantom types to enforce setup ordering:

- `Run<()>`: Unprepared run, hasn't created directories yet
- `Run<PathBuf>`: Prepared run with directory created, ready to execute

**Phase-based Execution**: Contexts are organized into phases:

- **Pre-phase**: Captured before command execution (e.g., git state, input files)
- **In-phase**: Watchers that monitor during execution (e.g., time watcher)
- **Post-phase**: Captured after command execution (e.g., output files, final state)

### Configuration Flow

1. CLI parses command-line arguments and loads `capsula.toml`
2. Config file is deserialized into `CapsulaConfig` structure
3. For each context in the config:
   - Registry looks up factory by type name
   - Factory deserializes context-specific fields from JSON
   - Factory creates a `Box<dyn ContextErased>` instance
4. Runtime params (phase, run_dir, project_root) are passed to each context
5. Each context's `run_erased()` method is called, returning captured data

### Output Structure

All captured context data is stored in the vault with this structure:

```
.capsula/{vault-name}/{date}/{time}-{name}--{ulid}/
├── metadata.json  # Run metadata (ID, name, command, timestamp)
├── pre.json       # Pre-phase context captures
├── run.json       # Command output (exit code, stdout, stderr, duration)
└── post.json      # Post-phase context captures
```

Each context output includes a `__meta` field with `success` status and `index` position from config.

### Error Handling

The codebase uses a layered error approach:

- `CapsulaError`: Top-level error type in capsula-core
- Crate-specific errors (e.g., `ConfigError`, `RegistryError`) that convert to `CapsulaError`
- The CLI uses `anyhow::Result` for error context and user-friendly messages
- Failed context captures emit warnings but don't abort the run (except when `abort_on_dirty` or similar flags are set)

### Environment Variables During Execution

When `capsula run` executes a command, these environment variables are set:

- `CAPSULA_RUN_ID`: ULID of the run
- `CAPSULA_RUN_NAME`: Generated human-readable name
- `CAPSULA_RUN_DIRECTORY`: Path to the run directory
- `CAPSULA_RUN_TIMESTAMP`: ISO 8601 timestamp
- `CAPSULA_RUN_COMMAND`: Shell-quoted command string

## Adding a New Context Type

To add a new context type:

1. Create a new crate `capsula-{type}-context` in `crates/`
2. Implement the `Context` trait with your capture logic
3. Implement the `Captured` trait for your output type
4. Create a factory implementing `ContextFactory`
5. Add a feature flag in workspace `Cargo.toml`
6. Register the factory in `capsula-registry::standard_registry()` with feature gate
7. Add dependency in `capsula-registry/Cargo.toml` with optional flag
