# Capsula

> [!WARNING]
> This project is in early development. The CLI interface and configuration format may change in future releases.

A powerful CLI tool for running hooks and capturing their output before and after your command executions. Capsula automatically records the state of your project environment before and after running commands, making your workflows reproducible and auditable.

> [!NOTE]
> The Python version of Capsula is deprecated and can be found at the `python` branch of this repository.

## Features

- 📸 **Context Capture**: Automatically capture git state, file contents, environment variables, and more
- 🔄 **Reproducible Runs**: Complete record of execution hook for debugging and auditing
- 🛡️ **Safety Checks**: Prevent execution on dirty repositories or other unsafe conditions
- 📊 **Structured Output**: JSON-formatted capture data for easy processing
- 🔧 **Extensible**: Multiple built-in hooks with clean error handling

## Installation

### Install from crates.io (recommended)

```bash
cargo install capsula-cli --locked
```

### Install from the GitHub repository

```bash
cargo install --git https://github.com/shunichironomura/capsula --locked capsula-cli
```

## Quick Start

1. **Create a configuration file** (`capsula.toml`) in your project root:

```toml
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "repo-name"
path = "."

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.json"
mode = "copy"
hash = "sha256"
```

2. **Run a command with hooks**:

```bash
capsula run python train_model.py
```


## Configuration

### Basic Structure

The `capsula.toml` configuration file defines:

- **Vault**: Where to store captured data
- **Phases**: Pre-run and post-run hooks

```toml
[vault]
name = "project-name"        # Vault identifier
path = ".capsula"           # Storage path (optional, defaults to .capsula/{name})

[pre-run]                 # Pre-execution hooks
[[pre-run.hooks]]
id = "capture-git-repo"
# ... hook configuration

[post-run]                # Post-execution hooks
[[post-run.hooks]]
id = "capture-file"
# ... hook configuration
```

### Available Hook Types

#### Git Hook

Captures git repository state including commit hash and cleanliness check.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "repo-name"          # Hook name
path = "."                  # Repository path
allow_dirty = false         # Allow uncommitted changes (default: false)
```

**Output Example:**

```json
{
  "__meta": {
    "config": {
      "name": "repo-name",
      "path": ".",
      "allow_dirty": false
    },
    "id": "capture-git-repo",
    "success": true
  },
  "working_dir": "/path/to/repo",
  "sha": "abc123...",
  "is_dirty": false,
  "abort_on_dirty": false
}
```

#### Current Working Directory

Captures the current working directory path.

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

**Output Example:**

```json
{
  "__meta": {
    "config": {},
    "id": "capture-cwd",
    "success": true
  },
  "cwd": "/current/working/directory"
}
```

#### File Hook

Captures file contents and/or metadata.

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "config.json"        # File pattern to capture
mode = "copy"               # Capture mode ("copy", "move", or "none". default: "copy")
hash = "sha256"             # Calculate file hash ("sha256" or "none". default: "sha256")
```

**Output Example:**

```json
{
  "__meta": {
    "config": {
      "glob": "config.json",
      "mode": "copy",
      "hash": "sha256"
    },
    "id": "capture-file",
    "success": true
  },
  "files": [
    {
      "path": "/path/to/config.json",
      "copied_path": "/vault/run-dir/config.json",
      "hash": "sha256:abc123..."
    }
  ]
}
```

#### Environment Variables Hook

Captures specified environment variables.

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "HOME"                 # Variable name to capture
```

**Output Example:**

```json
{
  "__meta": {
    "config": {
      "name": "HOME"
    },
    "id": "capture-env",
    "success": true
  },
  "value": "/home/user"
}
```

#### Command Hook

Captures output of shell commands.

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["uname", "-a"]
abort_on_failure  = false  # Abort if command fails (default: false)
```

**Output Example:**

```json
{
  "__meta": {
    "config": {
      "command": ["rustc", "--version"],
      "abort_on_failure": false
    },
    "id": "capture-command",
    "success": true
  },
  "status": 0,
  "stdout": "rustc 1.91.0 (f8297e351 2025-10-28)\n",
  "stderr": "",
  "abort_requested": false
}
```

#### Machine Hook

Captures system information like CPU, memory, and OS details.

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

**Output Example:**

```json
{
  "__meta": {
    "config": {},
    "id": "capture-machine",
    "success": true
  },
  "hostname": "hostname.local",
  "os": "Darwin",
  "os_version": "26.0.1",
  "kernel_version": "25.0.0",
  "architecture": "aarch64",
  "total_memory": 137438953472,
  "cpus": [
    {
      "name": "1",
      "brand": "Apple M3 Max",
      "vender_id": "Apple",
      "frequency_mhz": 4056
    }
  ]
}
```

## CLI Usage

### Commands

#### `capsula run <command>`

Execute a command with full hook capture.

```bash
# Run with default config
capsula run python script.py

# Run with custom config
capsula run --config my-config.toml python script.py

# Run with arguments
capsula run python train.py --epochs 100 --lr 0.01
```

**Behavior:**

1. Runs pre-run hooks and saves their outputs to vault
2. Checks for abort conditions (e.g., dirty git repo)
3. Executes the command if safe, aborts otherwise
4. Runs post-run hooks and saves their outputs to vault

**Environment Variables:**

When executing a command with `capsula run`, the following environment variables are automatically set and available to your command:

| Variable | Description | Example |
|----------|-------------|---------|
| `CAPSULA_RUN_ID` | Unique ULID identifier for this run | `01K8WSYC91YAE21R7CWHQ4KYN2` |
| `CAPSULA_RUN_NAME` | Human-readable generated name | `chubby-back` |
| `CAPSULA_RUN_DIRECTORY` | Absolute path to the run directory in the vault | `/path/to/.capsula/vault-name/2025-10-31/093525-chubby-back` |
| `CAPSULA_RUN_TIMESTAMP` | ISO 8601 timestamp of when the run started | `2025-10-31T09:35:25.473+00:00` |
| `CAPSULA_RUN_COMMAND` | Shell-quoted string of the executed command | `python train.py --epochs 100` |
| `CAPSULA_PRE_RUN_OUTPUT_PATH` | Path to the pre-run output JSON file | `/path/to/.capsula/vault-name/.../pre-run.json` |

> [!CAUTION]
> While `CAPSULA_RUN_DIRECTORY` is available, it is **not recommended** to write files directly to this directory. Instead, output files to your project root and capture them using the `capture-file` hook in the post-run phase. This approach ensures files are properly tracked and managed by Capsula's file handling system.

These variables can be used within your scripts to access run metadata:

```python
# Example: Embed run name in matplotlib figures for traceability
import os
import matplotlib.pyplot as plt

run_name = os.environ.get('CAPSULA_RUN_NAME')

# Create your plot
plt.plot([1, 2, 3, 4], [1, 4, 2, 3])
plt.title('Experiment Results')

# Add run name to the figure - useful when copying plots to presentations
plt.figtext(0.99, 0.01, f'Run: {run_name}',
            ha='right', va='bottom', fontsize=8, alpha=0.7)

# Save to project root, then capture with post-run hook
plt.savefig('results.png')
```

```toml
# Configure a post-run hook to capture the output file
[[post-run.hooks]]
id = "capture-file"
glob = "results.png"
mode = "move"  # Move the file to the vault
```

#### `capsula list`

List all captured runs in the vault.

```bash
# List runs with default config
capsula list

# List runs with custom config
capsula list --config my-config.toml
```

**Example Output:**

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-10-31 09:35:29  kind-year             echo hello
2025-10-31 09:35:28  smelly-apparel        echo hello
2025-10-31 09:35:26  clear-waste           echo hello
2025-10-31 09:30:15  cheap-trip            echo this is a very long command with many argu...
```

The output shows:
- **Timestamp**: UTC time when the command was executed
- **Name**: Human-readable generated name for the run
- **Command**: The command that was executed (truncated if too long)


## Output Structure

### Metadata

Every hook output includes metadata for traceability:

```json
{
  "__meta": {
    "config": {},    // Configuration used for this hook
    "id": "capture-cwd",  // Hook ID from configuration
    "success": true  // Capture success status
  }
  // ... hook-specific data
}
```

### Vault Structure

Captured data is organized in the vault:

```
.capsula/
└── vault-name/
    └── 2025-10-31/                    # Date-based directory (YYYY-MM-DD, UTC)
        └── 093525-chubby-back/        # Unique run directory (HHMMSS-run-name)
            ├── _capsula/              # Capsula metadata directory
            │   ├── metadata.json      # Run metadata (ID, name, command, timestamp)
            │   ├── pre-run.json       # Pre-phase hook outputs (array)
            │   ├── command.json       # Command execution results
            │   └── post-run.json      # Post-phase hook outputs (array)
            └── [captured files]       # Files copied by file hooks
```

**metadata.json** contains run information:

```json
{
  "id": "01K8WSYC91YAE21R7CWHQ4KYN2",
  "name": "chubby-back",
  "command": ["echo", "hello"],
  "timestamp": "2025-10-31T09:35:25.473+00:00",
  "run_dir": "/path/to/.capsula/vault-name/2025-10-31/093525-chubby-back"
}
```

**command.json** contains command execution results:

```json
{
  "exit_code": 0,
  "stdout": "hello\n",
  "stderr": "",
  "duration": {
    "secs": 0,
    "nanos": 1986042
  }
}
```

## License

This project is licensed under either of the MIT license or the Apache License 2.0 at your option.
