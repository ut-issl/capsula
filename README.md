# Capsula

> [!WARNING]
> This project is in early development. The CLI interface and configuration format may change in future releases.

A powerful CLI tool for running hooks and capturing their output before and after your command executions. Capsula automatically records the state of your project environment before and after running commands, making your workflows reproducible and auditable.

> [!NOTE]
> The Python version of Capsula is deprecated and can be found at the main branch of this repository.

## Features

- 📸 **Context Capture**: Automatically capture git state, file contents, environment variables, and more
- 🔄 **Reproducible Runs**: Complete record of execution hook for debugging and auditing
- 🛡️ **Safety Checks**: Prevent execution on dirty repositories or other unsafe conditions
- 📊 **Structured Output**: JSON-formatted capture data for easy processing
- 🔧 **Extensible**: Multiple built-in hooks with clean error handling

## Installation

### Install from crates.io

```bash
cargo install capsula-cli --locked
```

### Install from the GitHub repository

```bash
cargo install --git https://github.com/shunichironomura/capsula --branch rust --locked capsula-cli
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
  "__meta": { "success": true, "index": 0 },
  "id": "capture-git-repo",
  "name": "repo-name",
  "working_dir": "/path/to/repo",
  "sha": "abc123...",
  "is_dirty": false,
  "abort_on_dirty": false
}
```

#### Current Working Directory

Captures the current working directory path.

```toml
[[pre-run.contexts]]
id = "capture-cwd"
```

**Output Example:**

```json
{
  "__meta": { "success": true, "index": 1 },
  "id": "capture-cwd",
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
  "__meta": { "success": true, "index": 2 },
  "id": "capture-file",
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
  "__meta": { "success": true, "index": 3 },
  "id": "capture-env",
  "name": "HOME",
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

#### Machine Hook

Captures system information like CPU, memory, and OS details.

```toml
[[pre-run.hooks]]
id = "capture-machine"
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


## Output Structure

### Metadata

Every hook output includes metadata for traceability:

```json
{
  "__meta": {
    "success": true, // Capture success status
    "index": 0 // Position in configuration (0-based)
  }
  // ... hook-specific data
}
```

### Vault Structure

Captured data is organized in the vault:

```
.capsula/
└── vault-name/
    └── 2024-01-15/ # Date-based directory (YYYY-MM-DD, UTC)
        └── 143022-example-run/ # Unique run directory (timestamp + run name)
            ├── metadata.json    # Run metadata
            ├── pre.json        # Pre-phase hooks
            ├── run.json        # Command output, exit code, duration
            └── post.json       # Post-phase hooks
```
