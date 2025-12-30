# Capsula

**A powerful CLI tool for capturing and preserving the context of command executions**

Capsula is a Rust-based tool that records the state of your project environment before and after running commands. It's perfect for reproducibility, auditing, and understanding exactly what happened during a command execution.

## Why Capsula?

- **Reproducibility**: Capture the complete context of your experiments and builds
- **Auditing**: Track what changed when you ran specific commands
- **Debugging**: Understand the environment state when issues occur
- **Collaboration**: Share complete execution context with teammates

## Key Features

### Pre and Post-Run Hooks

Execute hooks before and after your commands to capture:

- Git repository state (commit hash, dirty status)
- Environment variables
- File contents and hashes
- System information (CPU, memory, OS)
- Custom command outputs

### Organized Output

All captured data is stored in a structured directory:

```
.capsula/{vault-name}/{YYYY-MM-DD}/{HHMMSS-name}/
├── _capsula/
│   ├── metadata.json
│   ├── pre-run.json
│   ├── command.json
│   └── post-run.json
└── [captured files]
```

### Flexible Configuration

Configure hooks using a simple TOML file:

```toml
[vault]
name = "my-experiments"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-repo"
path = "."
allow_dirty = true

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
hash = "sha256"
```

### Environment Variables

Commands executed with Capsula have access to special environment variables:

- `CAPSULA_RUN_ID`: Unique identifier for the run
- `CAPSULA_RUN_DIRECTORY`: Path to the run directory
- `CAPSULA_RUN_TIMESTAMP`: ISO 8601 timestamp
- And more...

## Quick Start

Install Capsula:

```bash
cargo install --path crates/capsula-cli --locked
```

Run a command with Capsula:

```bash
capsula run python train_model.py
```

List previous runs:

```bash
capsula list
```

## What's Next?

- [Getting Started](getting-started.md) - Installation and first steps
- [Configuration](configuration.md) - Learn how to configure Capsula
- [Architecture](architecture.md) - Understand how Capsula works
- [Development](development.md) - Contribute to Capsula

## Project Status

Capsula is written in Rust and actively developed. The Python version found on the main branch is deprecated.
