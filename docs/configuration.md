# Configuration

Capsula is configured using a `capsula.toml` file. This guide explains all configuration options and how to use them.

## Configuration File Location

Capsula looks for `capsula.toml` in the following order:

1. **Path specified with `--config` flag**
   ```bash
   capsula --config /path/to/custom.toml run python script.py
   ```

2. **Current directory**
   ```bash
   ./capsula.toml
   ```

3. **Parent directories** (walking up the directory tree)
   ```bash
   ../capsula.toml
   ../../capsula.toml
   ...
   ```

!!! tip
    Place `capsula.toml` in your project root so it works from any subdirectory.

## Basic Structure

Every configuration file has this basic structure:

```toml
[vault]
name = "vault-name"

[[pre-run.hooks]]
id = "hook-type"
# hook configuration...

[[post-run.hooks]]
id = "hook-type"
# hook configuration...
```

## Vault Configuration

The `[vault]` section defines where Capsula stores captured data.

### `name` (required)

The vault name creates a subdirectory under `.capsula/`.

```toml
[vault]
name = "ml-experiments"
```

This creates: `.capsula/ml-experiments/`

!!! tip "Naming vaults"
    Use descriptive names like `ml-experiments`, `daily-builds`, or `data-processing` to organize different types of runs.

### `path` (optional)

By default, vaults are stored in `.capsula/{name}/`. You can specify a custom path:

```toml
[vault]
name = "experiments"
path = "/absolute/path/to/vault"
```

Or a relative path:

```toml
[vault]
name = "experiments"
path = "custom/vault/location"
```

## Environment Variables from Files

Load environment variables from a `.env` file before running hooks and commands.

### `dotenv` (optional)

```toml
dotenv = ".env"
```

Or with an absolute path:

```toml
dotenv = "/absolute/path/to/.env"
```

**Example `.env` file:**

```bash
DATABASE_URL=postgresql://localhost/mydb
API_KEY=secret-key-here
SLACK_BOT_TOKEN=xoxb-...
```

**Behavior:**

- File is loaded before running any hooks or the main command
- Variables are available to all hooks and the executed command
- If the file fails to load, a warning is shown but execution continues
- Relative paths are resolved from the directory containing `capsula.toml`

!!! warning "Security"
    Add `.env` to your `.gitignore` to avoid committing secrets!

## Hook Configuration

Hooks are executed in the order they appear in your configuration file.

### Pre-Run Hooks

Run before your command executes:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"
```

### Post-Run Hooks

Run after your command completes:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "#results"
```

## Available Hook Types

### capture-cwd

Captures the current working directory.

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

**No configuration needed.**

[Learn more about capture-cwd →](hooks/capture-cwd.md)

### capture-env

Captures environment variables.

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"
```

**Required:**

- `name` - Environment variable name

[Learn more about capture-env →](hooks/capture-env.md)

### capture-git-repo

Captures git repository state.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

**Required:**

- `path` - Path to repository (`.` for current directory)

**Optional:**

- `allow_dirty` - Allow uncommitted changes (default: `false`)
  - If `false`, Capsula aborts when the repository has uncommitted changes

[Learn more about capture-git-repo →](hooks/capture-git-repo.md)

### capture-file

Captures files by copying, moving, or hashing them.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.txt"
mode = "copy"
hash = "sha256"
```

**Required:**

- `glob` - File pattern (e.g., `"*.txt"`, `"results/**/*.png"`)

**Optional:**

- `mode` - How to handle files (default: `"copy"`)
  - `"copy"` - Copy files to vault
  - `"move"` - Move files to vault
  - `"none"` - Don't copy files (just hash)
- `hash` - Hash algorithm (default: `"sha256"`)
  - `"sha256"` - Compute SHA-256 hash
  - `"none"` - Don't compute hash

[Learn more about capture-file →](hooks/capture-file.md)

### capture-machine

Captures system information.

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

**No configuration needed.**

Captures: hostname, OS, CPU info, memory.

[Learn more about capture-machine →](hooks/capture-machine.md)

### capture-command

Runs a command and captures its output.

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["python", "--version"]
abort_on_failure = false
```

**Required:**

- `command` - Command as array (e.g., `["ls", "-la"]`)

**Optional:**

- `abort_on_failure` - Abort if command fails (default: `false`)

[Learn more about capture-command →](hooks/capture-command.md)

### notify-slack

Sends notifications to Slack.

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#general"
attachment_globs = ["*.png"]
```

**Required:**

- `channel` - Slack channel (e.g., `"#general"`) or channel ID

**Optional:**

- `token` - Slack bot token (defaults to `SLACK_BOT_TOKEN` env var)
- `attachment_globs` - File patterns to attach (up to 10 files)

[Learn more about notify-slack →](hooks/notify-slack.md)

## Complete Configuration Example

Here's a comprehensive example showing all sections:

```toml title="capsula.toml"
# Load environment variables
dotenv = ".env"

# Configure vault
[vault]
name = "research-experiments"

# Pre-run hooks: capture initial state
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Abort if repo is dirty

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

# Post-run hooks: capture results
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.json"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "models/*.pkl"
mode = "none"
hash = "sha256"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png", "plots/*.pdf"]
```

## Configuration Tips

### Organizing Multiple Vaults

Use different vaults for different purposes:

```toml title="experiments.toml"
[vault]
name = "experiments"
# ... hooks for experiments
```

```toml title="builds.toml"
[vault]
name = "builds"
# ... hooks for builds
```

Run with specific config:

```bash
capsula --config experiments.toml run python train.py
capsula --config builds.toml run cargo build
```

### Hook Execution Order Matters

Hooks run in the order you define them. This matters when:

- **Using `mode = "move"` with files** - The file won't exist for later hooks
- **Checking preconditions** - Put validation hooks first

Example:

```toml
# BAD: File is moved before Slack hook can attach it
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]  # File already moved!
```

```toml
# GOOD: Slack attaches file, then it's moved
[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]

[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"
```

### Reusing Hook Types

You can use the same hook type multiple times:

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "HOME"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"
```

### Glob Patterns

File glob patterns support:

- `*` - Match anything except `/`
- `**` - Match anything including `/`
- `?` - Match single character
- `[abc]` - Match any character in set

Examples:

```toml
glob = "*.txt"              # All .txt files in current directory
glob = "results/**/*.csv"   # All .csv files in results/ and subdirectories
glob = "data_?.json"        # data_1.json, data_2.json, etc.
glob = "output.[tT][xX][tT]"  # output.txt, output.TXT, etc.
```

## Validation and Error Handling

### Configuration Errors

If your configuration has errors, Capsula will show a clear error message:

```bash
$ capsula run echo test
Error: Failed to parse configuration file
  --> capsula.toml:5:1
  |
5 | [[pre-run.hooks]
  | ^^^^^^^^^^^^^^^^ Missing closing bracket
```

### Hook Errors

If a hook fails, Capsula logs a warning and continues:

```
WARN: Hook 'capture-file' failed: File not found: missing.txt
```

The error is also saved in the hook's JSON output:

```json
{
  "__meta": {
    "id": "capture-file",
    "success": false,
    "error": "File not found: missing.txt"
  }
}
```

### Fatal Errors

Some conditions cause Capsula to abort:

- Configuration file cannot be parsed
- Run directory cannot be created
- A hook requests abort (e.g., dirty git repo when `allow_dirty = false`)


## Next Steps

- [Configuration Guide](configuration.md) - Learn about all configuration options
- [Hooks Reference](hooks.md) - Explore available hooks
- [CLI Reference](cli-reference.md) - Complete command reference

