# Hooks Overview

Hooks are the core mechanism for capturing context in Capsula. They execute before (pre-run) or after (post-run) your command.

## What are Hooks?

Hooks are small, composable units that:

- Capture specific pieces of information about your environment
- Execute at defined points in the command lifecycle
- Produce structured JSON output
- Can optionally abort execution based on conditions

## Hook Phases

### Pre-Run Hooks

Execute **before** your command runs. Use them to:

- Capture initial state
- Validate conditions (e.g., clean git repository)
- Document input configuration

### Post-Run Hooks

Execute **after** your command completes. Use them to:

- Capture results and outputs
- Archive generated files
- Send notifications
- Run analysis commands

## Available Hooks

### Capture Hooks

- **[capture_cwd](hooks/capture-cwd.md)** - Current working directory
- **[capture_env](hooks/capture-env.md)** - Environment variables
- **[capture_git_repo](hooks/capture-git-repo.md)** - Git repository state
- **[capture_file](hooks/capture-file.md)** - File content and hashes
- **[capture_machine](hooks/capture-machine.md)** - System information
- **[capture_command](hooks/capture-command.md)** - Shell command output

### Notification Hooks

- **[notify_slack](hooks/notify-slack.md)** - Slack notifications

## Hook Configuration

Hooks are configured in `capsula.toml`:

```toml
[vault]
name = "my-vault"

[[pre_run]]
type = "capture_git_repo"
allow_dirty = false

[[post_run]]
type = "capture_file"
path = "output.txt"
copy = true
```

## Hook Execution

### Execution Order

Hooks execute sequentially in the order they appear in the configuration:

```toml
[[pre_run]]
type = "capture_git_repo"  # 1st

[[pre_run]]
type = "capture_env"       # 2nd

[[pre_run]]
type = "capture_file"      # 3rd
path = "config.yaml"
```

### Error Handling

**Non-Fatal Errors**: Most hook errors are logged but don't stop execution:

```json
{
  "__meta": {
    "id": "capture_file",
    "success": false,
    "error": "File not found: missing.txt"
  }
}
```

**Fatal Errors**: Some hooks can abort the run:

- `capture_git_repo` with `allow_dirty = false` when repository is dirty

### Hook Output Format

All hooks produce JSON output with a `__meta` field:

```json
{
  "__meta": {
    "id": "capture_git_repo",
    "config": { "allow_dirty": true },
    "success": true
  },
  "commit": "a1b2c3d4...",
  "branch": "main",
  "dirty": false
}
```

## Quick Start Example

```toml
[vault]
name = "experiments"

# Pre-run: Capture initial state
[[pre_run]]
type = "capture_git_repo"

[[pre_run]]
type = "capture_env"
include = ["PATH", "PYTHONPATH"]

[[pre_run]]
type = "capture_file"
path = "config.yaml"
copy = true

# Post-run: Capture results
[[post_run]]
type = "capture_file"
path = "results/output.json"
copy = true
compute_hash = true

[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Experiment {run_name} completed!"
```

## Next Steps

- Browse individual hook documentation in the menu
- [Configuration Guide](configuration.md) - Complete configuration reference
- [Development](development.md) - Create custom hooks
