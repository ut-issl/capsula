# Configuration

Capsula is configured using a `capsula.toml` file in your project directory.

## Basic Structure

```toml
[vault]
name = "my-vault"

[[pre_run]]
type = "hook_type"
# hook-specific configuration

[[post_run]]
type = "hook_type"
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
[[pre_run]]
type = "capture_git_repo"
allow_dirty = true

[[pre_run]]
type = "capture_env"
include = ["PATH", "HOME"]
```

### Post-Run Hooks

Post-run hooks are executed after your command completes. Use them to capture results.

```toml
[[post_run]]
type = "capture_file"
path = "output.txt"
copy = true
```

## Hook Configuration

Each hook type has its own configuration options.

### `capture_cwd`

Captures the current working directory.

```toml
[[pre_run]]
type = "capture_cwd"
```

**Output**: Working directory path

**Config**: None

### `capture_env`

Captures environment variables.

```toml
[[pre_run]]
type = "capture_env"
include = ["PATH", "PYTHONPATH", "HOME"]
exclude = ["SECRET_KEY"]
```

**Config**:

- `include` (optional): List of environment variables to capture. If not specified, captures all.
- `exclude` (optional): List of environment variables to exclude from capture.

**Output**: Dictionary of environment variable names and values

### `capture_git_repo`

Captures Git repository state.

```toml
[[pre_run]]
type = "capture_git_repo"
allow_dirty = false
```

**Config**:

- `allow_dirty` (optional, default: `true`): If `false`, aborts the run if the repository has uncommitted changes.

**Output**:

- Commit hash
- Branch name
- Remote URL
- Dirty status (uncommitted changes)

### `capture_file`

Captures file content or computes hash.

```toml
[[post_run]]
type = "capture_file"
path = "results/model.pkl"
copy = true
compute_hash = true
algorithm = "sha256"
```

**Config**:

- `path` (required): Path to the file (relative to project root)
- `copy` (optional, default: `false`): Copy the file to the run directory
- `compute_hash` (optional, default: `false`): Compute file hash
- `algorithm` (optional, default: `"sha256"`): Hash algorithm (`"sha256"`, `"md5"`, `"sha1"`)

**Output**:

- File path
- File size
- Hash (if `compute_hash = true`)
- Copied path (if `copy = true`)

### `capture_machine`

Captures system information.

```toml
[[pre_run]]
type = "capture_machine"
```

**Config**: None

**Output**:

- OS name and version
- CPU information
- Total memory
- Hostname

### `capture_command`

Executes a shell command and captures its output.

```toml
[[post_run]]
type = "capture_command"
command = "ls -la results/"
shell = "/bin/bash"
```

**Config**:

- `command` (required): Shell command to execute
- `shell` (optional, default: `"/bin/sh"`): Shell to use for execution

**Output**:

- Command executed
- Exit code
- Standard output
- Standard error

### `notify_slack`

Sends a notification to Slack.

```toml
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Experiment completed!"
```

**Config**:

- `webhook_url_env` (required): Environment variable containing the Slack webhook URL
- `message` (optional): Message to send. Supports template variables.

**Template Variables**:

- `{run_id}`: Run ID
- `{run_name}`: Run name
- `{command}`: Command executed
- `{timestamp}`: Run timestamp

**Output**: Slack API response

## Complete Example

```toml
[vault]
name = "research-experiments"

# Capture initial state
[[pre_run]]
type = "capture_git_repo"
allow_dirty = false  # Fail if repo has uncommitted changes

[[pre_run]]
type = "capture_env"
include = [
    "PATH",
    "PYTHONPATH",
    "CUDA_VISIBLE_DEVICES",
    "OMP_NUM_THREADS",
]

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_file"
path = "config.yaml"
copy = true
compute_hash = true

# Capture results
[[post_run]]
type = "capture_file"
path = "results/metrics.json"
copy = true

[[post_run]]
type = "capture_file"
path = "results/model.pkl"
compute_hash = true
algorithm = "sha256"

[[post_run]]
type = "capture_command"
command = "python -c 'import sys; print(sys.version)'"

[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Experiment {run_name} completed at {timestamp}"
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
