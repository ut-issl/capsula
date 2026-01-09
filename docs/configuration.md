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

Hooks are the core of Capsula - they capture information about your command execution. This section explains how to configure hooks and how they behave.

### Hook Execution

**Hooks run in order** - They execute sequentially in the order they appear in your configuration file.

**Most errors are non-fatal** - If a hook fails, Capsula:

- Logs a warning
- Records the error in the hook's JSON output (with `"success": false`)
- Continues executing remaining hooks

This ensures partial success is always recorded, which is valuable for debugging.

**Some hooks can abort execution** - Certain hooks can request to stop the run before your command executes. For example, `capture-git-repo` with `allow_dirty = false` will abort if the repository has uncommitted changes.

**Hook output format** - All hooks produce JSON output with a standard `__meta` field:

```json
{
  "__meta": {
    "id": "capture-cwd",
    "config": {},
    "success": true
  },
  "cwd": "/path/to/directory"
}
```

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

For a complete list of available hooks with descriptions, see the [Available Hooks](getting-started.md#available-hooks) table in the Getting Started guide.

Each hook has detailed documentation linked from that table with configuration options, examples, and use cases.

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
