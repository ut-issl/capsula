# Getting Started

This guide will help you install Capsula and run your first command.

## Prerequisites

- Rust toolchain (1.70 or later)
- Git (for repository state capture)

## Installation

### From Source

Clone the repository and install:

```bash
git clone https://github.com/ut-issl/capsula.git
cd capsula
cargo install --path crates/capsula-cli --locked
```

### Verify Installation

Check that Capsula is installed:

```bash
capsula --version
```

## Your First Run

### 1. Create a Configuration File

Create a `capsula.toml` file in your project directory:

```toml
[vault]
name = "my-project"

[[pre_run]]
type = "capture_cwd"

[[pre_run]]
type = "capture_git_repo"

[[post_run]]
type = "capture_command"
command = "echo 'Post-run complete'"
```

### 2. Run a Command

Execute a command with Capsula:

```bash
capsula run echo "Hello, Capsula!"
```

This will:

1. Create a run directory in `.capsula/my-project/{date}/{time-name}/`
2. Execute pre-run hooks (capture working directory and git state)
3. Run your command
4. Execute post-run hooks
5. Save all captured data

### 3. View the Output

Check the captured data:

```bash
# List all runs
capsula list

# View the latest run directory
ls -la .capsula/my-project/$(date +%Y-%m-%d)/
```

The run directory contains:

```
.capsula/my-project/2025-12-30/143022-chubby-back/
├── _capsula/
│   ├── metadata.json      # Run metadata (ID, command, timestamp)
│   ├── pre-run.json       # Pre-run hook outputs
│   ├── command.json       # Command execution results
│   └── post-run.json      # Post-run hook outputs
```

### 4. Inspect the JSON Files

View the metadata:

```bash
cat .capsula/my-project/*/latest/_capsula/metadata.json
```

View pre-run hook results:

```bash
cat .capsula/my-project/*/latest/_capsula/pre-run.json
```

## Common Use Cases

### Capture Python Script Execution

```toml
[vault]
name = "ml-experiments"

[[pre_run]]
type = "capture_git_repo"
allow_dirty = false  # Fail if repo is dirty

[[pre_run]]
type = "capture_env"
include = ["PYTHONPATH", "CUDA_VISIBLE_DEVICES"]

[[pre_run]]
type = "capture_file"
path = "config.yaml"
copy = true

[[post_run]]
type = "capture_file"
path = "results/metrics.json"
copy = true
```

Run your script:

```bash
capsula run python train.py --config config.yaml
```

### Capture Build Execution

```toml
[vault]
name = "builds"

[[pre_run]]
type = "capture_git_repo"

[[pre_run]]
type = "capture_command"
command = "rustc --version"

[[post_run]]
type = "capture_command"
command = "ls -lh target/release/"
```

Run your build:

```bash
capsula run cargo build --release
```

## Environment Variables

Your command has access to special environment variables:

```bash
capsula run bash -c 'echo "Run ID: $CAPSULA_RUN_ID"'
```

Available variables:

- `CAPSULA_RUN_ID`: Unique ULID for this run
- `CAPSULA_RUN_NAME`: Human-readable name (e.g., "chubby-back")
- `CAPSULA_RUN_DIRECTORY`: Absolute path to the run directory
- `CAPSULA_RUN_TIMESTAMP`: ISO 8601 timestamp
- `CAPSULA_RUN_COMMAND`: The command being executed
- `CAPSULA_PRE_RUN_OUTPUT_PATH`: Path to pre-run.json
- `CAPSULA_PROJECT_ROOT`: Project root directory

## Next Steps

- [Configuration](configuration.md) - Learn about all configuration options
- [Hooks](hooks.md) - Explore available hook types
- [Architecture](architecture.md) - Understand how Capsula works
- [Development](development.md) - Contribute to Capsula
