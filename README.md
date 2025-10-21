# Capsula

> [!WARNING]
> This project is in early development. The CLI interface and configuration format may change in future releases.

A powerful CLI tool for capturing and preserving the context of your command executions. Capsula automatically records the state of your project environment before and after running commands, making your workflows reproducible and auditable.

> [!NOTE]
> The Python version of Capsula is deprecated and can be found at the main branch of this repository.

## Features

- 📸 **Context Capture**: Automatically capture git state, file contents, environment variables, and more
- 🔄 **Reproducible Runs**: Complete record of execution context for debugging and auditing
- 🛡️ **Safety Checks**: Prevent execution on dirty repositories or other unsafe conditions
- 📊 **Structured Output**: JSON-formatted capture data for easy processing
- 🔧 **Extensible**: Multiple built-in context types with clean error handling

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

[[phase.pre.contexts]]
id = "git"
name = "repo-name"
path = "."

[[phase.pre.contexts]]
id = "cwd"

[[phase.pre.contexts]]
id = "file"
glob = "config.json"
mode = "copy"
hash = "sha256"
```

2. **Run a command with context capture**:

```bash
capsula run python train_model.py
```

3. **Or capture context only**:

```bash
capsula capture --phase pre
```

## Configuration

### Basic Structure

The `capsula.toml` configuration file defines:

- **Vault**: Where to store captured data
- **Phases**: When to capture context (before/after command execution)
- **Contexts**: What information to capture

```toml
[vault]
name = "project-name"        # Vault identifier
path = ".capsula"           # Storage path (optional, defaults to .capsula/{name})

[phase.pre]                 # Pre-execution contexts
[[phase.pre.contexts]]
id = "git"
# ... context configuration

[phase.post]                # Post-execution contexts
[[phase.post.contexts]]
id = "file"
# ... context configuration
```

### Available Context Types

#### Git Context

Captures git repository state including commit hash and cleanliness check.

```toml
[[phase.pre.contexts]]
id = "git"
name = "repo-name"          # Context name
path = "."                  # Repository path
allow_dirty = false         # Allow uncommitted changes (default: false)
```

**Output Example:**

```json
{
  "__meta": { "success": true, "index": 0 },
  "id": "git",
  "name": "repo-name",
  "working_dir": "/path/to/repo",
  "sha": "abc123...",
  "is_dirty": false,
  "abort_on_dirty": false
}
```

#### Current Working Directory Context

Captures the current working directory path.

```toml
[[phase.pre.contexts]]
id = "cwd"
```

**Output Example:**

```json
{
  "__meta": { "success": true, "index": 1 },
  "id": "cwd",
  "cwd": "/current/working/directory"
}
```

#### File Context

Captures file contents and/or metadata.

```toml
[[phase.pre.contexts]]
id = "file"
glob = "config.json"        # File pattern to capture
mode = "copy"               # Capture mode ("copy", "move", or "none". default: "copy")
hash = "sha256"             # Calculate file hash ("sha256" or "none". default: "sha256")
```

**Output Example:**

```json
{
  "__meta": { "success": true, "index": 2 },
  "id": "file",
  "files": [
    {
      "path": "/path/to/config.json",
      "copied_path": "/vault/run-dir/config.json",
      "hash": "sha256:abc123..."
    }
  ]
}
```

**Note:** Copy and move modes require a run directory and only work during `capsula run` command, not standalone `capsula capture`.

#### Environment Variables Context

Captures specified environment variables.

```toml
[[phase.pre.contexts]]
id = "env"
name = "HOME"                 # Variable name to capture
```

**Output Example:**

```json
{
  "__meta": { "success": true, "index": 3 },
  "id": "env",
  "name": "HOME",
  "value": "/home/user"
}
```

#### Command Context

Captures output of shell commands.

```toml
[[phase.pre.contexts]]
id = "command"
command = ["uname", "-a"]
abort_on_failure  = false  # Abort if command fails (default: false)
```

#### Machine Context

Captures system information like CPU, memory, and OS details.

```toml
[[phase.pre.contexts]]
id = "machine"
```

## CLI Usage

### Commands

#### `capsula run <command>`

Execute a command with full context capture.

```bash
# Run with default config
capsula run python script.py

# Run with custom config
capsula run --config my-config.toml python script.py

# Run with arguments
capsula run python train.py --epochs 100 --lr 0.01
```

**Behavior:**

1. Captures pre-phase contexts and saves to vault
2. Checks for abort conditions (e.g., dirty git repo)
3. Executes the command if safe, aborts otherwise
4. Captures post-phase contexts and saves to vault

#### `capsula capture`

Capture context without running a command.

```bash
# Capture pre-phase contexts
capsula capture --phase pre

# Capture post-phase contexts
capsula capture --phase post
```

### Options

- `--config <path>`: Specify custom configuration file (default: `capsula.toml`)
- `--phase <phase>`: Specify phase for capture command (`pre` or `post`)

## Output Structure

### Metadata

Every context output includes metadata for traceability:

```json
{
  "__meta": {
    "success": true, // Capture success status
    "index": 0 // Position in configuration (0-based)
  }
  // ... context-specific data
}
```

### Vault Structure

Captured data is organized in the vault:

```
.capsula/
└── vault-name/
    └── 2024-01-15/ # Date-based directory (YYYY-MM-DD, UTC)
        └── 143022-example-run--01HKJM2K3L4M5N6P7Q8R9S/ # Unique run directory (timestamp + run name + ULID)
            ├── metadata.json    # Run metadata
            ├── pre.json        # Pre-phase contexts
            ├── run.json        # Command output, exit code, duration
            └── post.json       # Post-phase contexts
```
