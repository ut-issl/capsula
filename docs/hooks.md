# Hooks

Hooks are the core feature of Capsula - they tell Capsula what information to capture and when. This page explains how hooks work and provides an overview of all available hooks.

## What Are Hooks?

A hook is a small unit of functionality that captures a specific piece of information. For example:

- The `capture-cwd` hook captures your current working directory
- The `capture-git-repo` hook captures git repository state
- The `capture-file` hook captures files

Hooks are defined in your `capsula.toml` configuration file.

## Hook Phases

Hooks run in two phases:

### Pre-Run Phase

Runs **before** your command executes. Use pre-run hooks to:

- Capture the initial environment state
- Validate preconditions (like clean git state)
- Record input configurations
- Capture system information

Example:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
```

### Post-Run Phase

Runs **after** your command completes. Use post-run hooks to:

- Capture output files and results
- Run analysis commands
- Send notifications
- Archive generated artifacts

Example:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.png"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "#results"
```

## Available Hooks

### Capture Hooks

These hooks capture information about your environment and files.

#### [capture-cwd](hooks/capture-cwd.md)

Captures the current working directory.

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

**Use when:** You want to know where the command was run from.

---

#### [capture-env](hooks/capture-env.md)

Captures environment variables.

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"
```

**Use when:** Your command depends on specific environment variables.

---

#### [capture-git-repo](hooks/capture-git-repo.md)

Captures git repository state including commit hash and dirty status.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

**Use when:** You want to ensure reproducibility by recording the exact code version.

---

#### [capture-file](hooks/capture-file.md)

Captures files by copying, moving, or computing their hash.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
hash = "sha256"
```

**Use when:** You want to preserve input configurations or output results.

---

#### [capture-machine](hooks/capture-machine.md)

Captures system information like CPU, memory, and OS details.

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

**Use when:** Results might depend on hardware specifications.

---

#### [capture-command](hooks/capture-command.md)

Runs a command and captures its output.

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]
```

**Use when:** You want to record tool versions or run diagnostic commands.

---

### Notification Hooks

These hooks send notifications about your runs.

#### [notify-slack](hooks/notify-slack.md)

Sends notifications to Slack with optional file attachments.

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

**Use when:** You want to be notified when long-running commands complete.

---

## Hook Configuration Basics

### Basic Structure

Each hook is configured as a TOML table with an `id` field:

```toml
[[pre-run.hooks]]
id = "hook-type"
option1 = "value"
option2 = 123
```

### Multiple Hooks

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
name = "USER"
```

### Execution Order

Hooks execute in the order they appear in your configuration file:

```toml
[[pre-run.hooks]]
id = "capture-cwd"     # Runs 1st

[[pre-run.hooks]]
id = "capture-git-repo"  # Runs 2nd
path = "."

[[pre-run.hooks]]
id = "capture-machine"  # Runs 3rd
```

## Hook Output Format

All hooks produce JSON output with a standard format.

### Successful Hook Output

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

The `__meta` field contains:

- `id` - Hook ID from configuration
- `config` - Configuration used for this hook
- `success` - Whether the hook succeeded

### Failed Hook Output

When a hook fails, the error is recorded:

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {"glob": "missing.txt"},
    "success": false,
    "error": "File not found: missing.txt"
  }
}
```

!!! info "Non-Fatal Errors"
    Most hook errors are non-fatal - Capsula logs the error and continues with remaining hooks. This ensures partial success is always recorded.

## Error Handling

### Non-Fatal Errors

By default, hook failures don't stop execution:

- Error is logged as a warning
- Error is recorded in the hook's JSON output
- Remaining hooks continue to execute

This is useful for debugging - you can see what succeeded and what failed.

### Fatal Errors (Aborting)

Some hooks can request to abort the run:

**Example:** Git hook with dirty repository

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Abort if repo has uncommitted changes
```

If the repository is dirty, Capsula will:

1. Save the hook output (showing the dirty state)
2. Stop before running your command
3. Exit with an error

This prevents running experiments with uncommitted code changes.

## Hook Selection Guide

### For Reproducibility

Essential hooks for reproducible research:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "results/*"
mode = "copy"
```

### For Debugging

Useful hooks for troubleshooting:

```toml
[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "list"]
```

### For Notifications

Hooks for staying informed:

```toml
[[pre-run.hooks]]
id = "notify-slack"
channel = "#experiments"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

### For Auditing

Hooks for compliance and auditing:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-env"
name = "USER"

[[pre-run.hooks]]
id = "capture-file"
glob = "inputs/**/*"
mode = "copy"
hash = "sha256"

[[post-run.hooks]]
id = "capture-file"
glob = "outputs/**/*"
mode = "copy"
hash = "sha256"
```

## Common Patterns

### Pattern: Capture Input Config

Save the configuration before running:

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "config.json"
mode = "copy"
```

### Pattern: Capture and Verify Git State

Ensure clean git state:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

### Pattern: Archive Results

Save all output files:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/**/*"
mode = "move"
```

### Pattern: Record Tool Versions

Capture versions of tools used:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "list"]

[[pre-run.hooks]]
id = "capture-command"
command = ["git", "--version"]
```

### Pattern: Notify on Completion

Send Slack notification when done:

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["summary.txt", "plots/*.png"]
```

### Pattern: Hash Without Copying

Verify file integrity without copying:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "large-model.bin"
mode = "none"
hash = "sha256"
```

## Best Practices

### 1. Order Hooks Carefully

Put validation hooks first:

```toml
# Check git state first
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false

# Then capture other info
[[pre-run.hooks]]
id = "capture-machine"
```

### 2. Capture Inputs in Pre-Run

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
```

### 3. Capture Outputs in Post-Run

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*"
mode = "copy"
```

### 4. Use `mode = "copy"` for Shared Files

If multiple hooks need the same file, use `copy` not `move`:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "copy"  # File stays available

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]  # Can still access it
```

### 5. Use Glob Patterns Effectively

```toml
glob = "*.txt"              # All .txt in current dir
glob = "results/**/*.csv"   # All .csv in results/ tree
glob = "data_?.json"        # data_1.json, data_2.json, etc.
```

## Exploring Hooks

Click on any hook below to see detailed documentation:

<div class="grid cards" markdown>

-   **[capture-cwd](hooks/capture-cwd.md)**

    Capture current working directory

-   **[capture-env](hooks/capture-env.md)**

    Capture environment variables

-   **[capture-git-repo](hooks/capture-git-repo.md)**

    Capture git repository state

-   **[capture-file](hooks/capture-file.md)**

    Capture files with copy/move/hash

-   **[capture-machine](hooks/capture-machine.md)**

    Capture system information

-   **[capture-command](hooks/capture-command.md)**

    Run commands and capture output

-   **[notify-slack](hooks/notify-slack.md)**

    Send Slack notifications

</div>


## Next Steps

- [Configuration Guide](configuration.md) - Learn about all configuration options
- [Hooks Reference](hooks.md) - Explore available hooks
- [CLI Reference](cli-reference.md) - Complete command reference

