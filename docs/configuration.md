# Configuration

Capsula is configured using a `capsula.toml` file in your project directory.

## Basic Structure

```toml
[vault]
name = "my-vault"

[[pre-run.hooks]]
id = "hook_type"
# hook-specific configuration

[[post-run.hooks]]
id = "hook_type"
# hook-specific configuration
```

## Vault Configuration

The `[vault]` section defines where captured data is stored.

### `name` (required)

The name of the vault. This creates a subdirectory in `.capsula/`.

```toml
[vault]
name = "ml-experiments"
```

Output location: `.capsula/ml-experiments/{date}/{time-name}/`

## Hooks

Hooks are executed in the order they appear in the configuration file.

### Pre-Run Hooks

Pre-run hooks are executed before your command runs. Use them to capture initial state.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-repo"
path = "."
allow_dirty = true

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "HOME"
```

### Post-Run Hooks

Post-run hooks are executed after your command completes. Use them to capture results.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

## Hook Configuration

Each hook type has its own configuration options.

### `capture_cwd`

Captures the current working directory.

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

**Output**: Working directory path

**Config**: None

### `capture-env`

Captures environment variables.

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"
```

**Config**:

- `name` (required): Environment variable name to capture

**Output**: Dictionary containing the variable name and its value (or null if not set)

### `capture-git-repo`

Captures Git repository state.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-repo"
path = "."
allow_dirty = false
```

**Config**:

- `name` (required): Name for this repository (used for patch file naming)
- `path` (required): Path to the repository (relative to project root, or absolute)
- `allow_dirty` (optional, default: `false`): If `false`, aborts the run if the repository has uncommitted changes

**Output**:

- Working directory path
- Commit SHA
- Dirty status (uncommitted changes)
- Patch file (if dirty)

### `capture-file`

Captures file content or computes hash.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/model.pkl"
mode = "copy"
hash = "sha256"
```

**Config**:

- `glob` (required): Glob pattern to match files (e.g., "*.txt", "results/**/*.pkl")
- `mode` (optional, default: `"copy"`): File handling mode - `"copy"`, `"move"`, or `"none"`
- `hash` (optional, default: `"sha256"`): Hash algorithm - `"sha256"` or `"none"`

**Output**:

- File path
- Copied path (if mode is `"copy"` or `"move"`)
- Hash (if hash is `"sha256"`)

### `capture_machine`

Captures system information.

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

**Config**: None

**Output**:

- OS name and version
- CPU information
- Total memory
- Hostname

### `capture-command`

Executes a shell command and captures its output.

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["ls", "-la", "results/"]
abort_on_failure = false
```

**Config**:

- `command` (required): Array of command and arguments (e.g., `["python", "--version"]`)
- `abort_on_failure` (optional, default: `false`): If `true`, aborts the run if command exits with non-zero status

**Output**:

- Standard output
- Standard error
- Exit status code

### `notify-slack`

Sends a notification to Slack.

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = ["*.png", "results/*.jpg"]
```

**Config**:

- `channel` (required): Slack channel ID (e.g., "C1234567890")
- `token` (optional): Slack bot token (defaults to `SLACK_BOT_TOKEN` environment variable)
- `attachment_globs` (optional): Array of glob patterns for files to attach (up to 10 files)

**Output**: Slack API response with attachment information

## Complete Example

```toml
[vault]
name = "research-experiments"

# Capture initial state
[[pre-run.hooks]]
id = "capture-git-repo"
name = "research-repo"
path = "."
allow_dirty = false  # Fail if repo has uncommitted changes

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-env"
name = "OMP_NUM_THREADS"

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"

# Capture results
[[post-run.hooks]]
id = "capture-file"
glob = "results/metrics.json"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "results/model.pkl"
mode = "none"
hash = "sha256"

[[post-run.hooks]]
id = "capture-command"
command = ["python", "-c", "import sys; print(sys.version)"]

[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = ["results/*.png"]
```

## Configuration Location

Capsula looks for `capsula.toml` in the following locations (in order):

1. Path specified with `--config` flag
2. Current directory
3. Parent directories (walking up the tree)

Example with custom config location:

```bash
capsula --config /path/to/custom.toml run python script.py
```

## Error Handling

### Non-Fatal Errors

If a hook fails, Capsula:

1. Logs a warning
2. Records the error in the hook's JSON output (`__meta.error` field)
3. Continues executing remaining hooks

This ensures partial success is always recorded.

### Fatal Errors

The run will be aborted if:

- Configuration file cannot be parsed
- Run directory cannot be created
- Command execution fails
- A hook requests abort (e.g., `capture_git_repo` with `allow_dirty = false` when repo is dirty)

## Next Steps

- [Hooks Reference](hooks.md) - Detailed hook documentation
- [Environment Variables](environment-variables.md) - Available environment variables
- [Development](development.md) - Add custom hooks
