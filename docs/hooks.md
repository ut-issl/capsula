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

[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-repo"
path = "."
allow_dirty = false

[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

## Hook Execution

### Execution Order

Hooks execute sequentially in the order they appear in the configuration:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"  # 1st
name = "my-repo"
path = "."

[[pre-run.hooks]]
id = "capture-env"       # 2nd
name = "PATH"

[[pre-run.hooks]]
id = "capture-file"      # 3rd
glob = "config.yaml"
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
[[pre-run.hooks]]
id = "capture-git-repo"
name = "experiment-repo"
path = "."

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"

# Post-run: Capture results
[[post-run.hooks]]
id = "capture-file"
glob = "results/output.json"
mode = "copy"
hash = "sha256"

[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = ["results/*.png"]
```

## Next Steps

- Browse individual hook documentation in the menu
- [Configuration Guide](configuration.md) - Complete configuration reference
- [Development](development.md) - Create custom hooks
