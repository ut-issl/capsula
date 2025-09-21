# Capsula

A powerful CLI tool for capturing and preserving the context of your command executions. Capsula automatically records the state of your project environment before and after running commands, making your workflows reproducible and auditable.

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
type = "git"
name = "git-state"
path = "."

[[phase.pre.contexts]]
type = "cwd"
name = "working-directory"

[[phase.pre.contexts]]
type = "file"
name = "config-file"
path = "config.json"
copy = true
hash = true
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
type = "git"
# ... context configuration

[phase.post]                # Post-execution contexts
[[phase.post.contexts]]
type = "file"
# ... context configuration
```

### Available Context Types

#### Git Context
Captures git repository state including commit hash and cleanliness check.

```toml
[[phase.pre.contexts]]
type = "git"
name = "repo-state"
path = "."                  # Repository path
allow_dirty = false         # Allow uncommitted changes (default: false)
```

**Output Example:**
```json
{
  "__meta": { "success": true, "index": 0 },
  "type": "git",
  "name": "repo-state",
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
type = "cwd"
name = "working-dir"
```

**Output Example:**
```json
{
  "__meta": { "success": true, "index": 1 },
  "type": "cwd",
  "cwd": "/current/working/directory"
}
```

#### File Context
Captures file contents and/or metadata.

```toml
[[phase.pre.contexts]]
type = "file"
name = "config-snapshot"
path = "config.json"        # File to capture
copy = true                 # Copy file contents (default: false)
hash = true                 # Calculate file hash (default: false)
```

**Output Example:**
```json
{
  "__meta": { "success": true, "index": 2 },
  "type": "file",
  "name": "config-snapshot",
  "path": "config.json",
  "content": "{ \"setting\": \"value\" }",
  "hash": "sha256:abc123..."
}
```

#### Environment Variables Context
Captures specified environment variables.

```toml
[[phase.pre.contexts]]
type = "env"
name = "environment"
vars = ["HOME", "USER", "PATH"]  # Variables to capture
```

**Output Example:**
```json
{
  "__meta": { "success": true, "index": 3 },
  "type": "env",
  "name": "environment",
  "value": {
    "HOME": "/home/user",
    "USER": "username",
    "PATH": "/usr/bin:/bin"
  }
}
```

#### Command Context
Captures output of shell commands.

```toml
[[phase.pre.contexts]]
type = "command"
name = "system-info"
command = ["uname", "-a"]
```

#### Machine Context
Captures system information like CPU, memory, and OS details.

```toml
[[phase.pre.contexts]]
type = "machine"
name = "system-specs"
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
1. Captures pre-phase contexts
2. Checks for abort conditions (e.g., dirty git repo)
3. Executes the command if safe
4. Captures post-phase contexts
5. Saves all data to vault

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
    "success": true,    // Capture success status
    "index": 0         // Position in configuration (0-based)
  },
  // ... context-specific data
}
```

### Error Handling
Failed contexts are:
- **Excluded** from JSON output (keeps data clean)
- **Reported** via console warnings with config index
- **Non-blocking** (other contexts still execute)

Example:
```bash
Warning: Failed to capture git (config index 1): Not a git repository
[
  {
    "__meta": { "success": true, "index": 0 },
    "type": "cwd",
    "cwd": "/path"
  },
  {
    "__meta": { "success": true, "index": 2 },
    "type": "file",
    "name": "config"
  }
]
```

### Vault Structure
Captured data is organized in the vault:

```
.capsula/
└── project-name/
    └── 2024-01-15/
        └── 143022-example-run--01HKJM2K3L4M5N6P7Q8R9S/
            ├── metadata.json    # Run metadata
            ├── pre.json        # Pre-phase contexts
            ├── run.json        # Command output
            └── post.json       # Post-phase contexts
```

## Safety Features

### Dirty Repository Protection
Git contexts can prevent execution on uncommitted changes:

```toml
[[phase.pre.contexts]]
type = "git"
allow_dirty = false  # Abort if repository is dirty
```

When `allow_dirty = false` and repository has uncommitted changes:
- Context is still captured with `"abort_on_dirty": true`
- Run is aborted before command execution
- Warning message is displayed

### Error Isolation
- Individual context failures don't stop the entire process
- Failed contexts are clearly reported with their configuration index
- Successful contexts continue to be captured and saved

## Examples

### Machine Learning Project
```toml
[vault]
name = "ml-experiments"

[[phase.pre.contexts]]
type = "git"
name = "code-state"
path = "."
allow_dirty = false

[[phase.pre.contexts]]
type = "file"
name = "config"
path = "config.yaml"
copy = true
hash = true

[[phase.pre.contexts]]
type = "env"
name = "environment"
vars = ["CUDA_VISIBLE_DEVICES", "PYTHONPATH"]

[[phase.post.contexts]]
type = "file"
name = "results"
path = "output/results.json"
copy = true
```

### Web Development
```toml
[vault]
name = "web-deploys"

[[phase.pre.contexts]]
type = "git"
name = "deployment-commit"
path = "."

[[phase.pre.contexts]]
type = "file"
name = "package-lock"
path = "package-lock.json"
hash = true

[[phase.pre.contexts]]
type = "command"
name = "node-version"
command = ["node", "--version"]
```

## Environment Variables

- `RUST_BACKTRACE=1`: Show detailed error backtraces
- `CAPSULA_VERBOSE=1`: Enable verbose output

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
