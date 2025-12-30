# capture_git_repo

Captures Git repository state including commit hash, branch, and dirty status.

## Configuration

```toml
# Allow execution with uncommitted changes
[[pre_run]]
type = "capture_git_repo"
allow_dirty = true

# Abort if repository has uncommitted changes
[[pre_run]]
type = "capture_git_repo"
allow_dirty = false
```

## Parameters

- `allow_dirty` (optional, default: `true`): If `false`, aborts the run when the repository has uncommitted changes.

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture_git_repo",
    "config": {
      "allow_dirty": false
    },
    "success": true
  },
  "commit": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "branch": "main",
  "remote": "https://github.com/ut-issl/capsula.git",
  "dirty": false,
  "status": "clean"
}
```

### Fields

- `commit` (string): Full commit hash (SHA-1)
- `branch` (string): Current branch name
- `remote` (string): Remote repository URL
- `dirty` (boolean): Whether repository has uncommitted changes
- `status` (string): Human-readable status ("clean" or "dirty")

## Use Cases

### Ensure Reproducibility

Require clean repository state for experiments:

```toml
[[pre_run]]
type = "capture_git_repo"
allow_dirty = false
```

If you try to run with uncommitted changes:

```bash
$ capsula run python experiment.py
Error: Git repository has uncommitted changes.
Commit or stash your changes, or set allow_dirty = true
```

### Track Commit for Results

Capture which commit produced specific results:

```toml
[[pre_run]]
type = "capture_git_repo"
allow_dirty = true
```

### Detect Code Changes During Execution

Compare repository state before and after:

```toml
[[pre_run]]
type = "capture_git_repo"

[[post_run]]
type = "capture_git_repo"
```

## Examples

### Strict Reproducibility Mode

```toml
[vault]
name = "research-experiments"

[[pre_run]]
type = "capture_git_repo"
allow_dirty = false  # Force clean repository

[[pre_run]]
type = "capture_env"
include = ["PATH"]
```

```bash
# This will fail if you have uncommitted changes
capsula run python train_model.py
```

### Relaxed Mode with Tracking

```toml
[vault]
name = "development-runs"

[[pre_run]]
type = "capture_git_repo"
allow_dirty = true  # Allow uncommitted changes
```

Output when repository is dirty:

```json
{
  "__meta": {
    "id": "capture_git_repo",
    "config": {
      "allow_dirty": true
    },
    "success": true
  },
  "commit": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "branch": "feature/new-model",
  "remote": "https://github.com/ut-issl/capsula.git",
  "dirty": true,
  "status": "dirty (2 modified, 1 untracked)"
}
```

### Compare Before and After

```toml
[vault]
name = "code-generation"

[[pre_run]]
type = "capture_git_repo"
allow_dirty = true

[[post_run]]
type = "capture_git_repo"
```

This captures whether your command modified any files:

```bash
capsula run python codegen.py
```

## Abort Behavior

When `allow_dirty = false` and the repository has uncommitted changes:

1. Capsula captures the pre-run hooks up to `capture_git_repo`
2. The `capture_git_repo` hook detects dirty state
3. Capsula writes partial `pre-run.json` with the abort reason
4. **Your command is NOT executed**
5. Capsula exits with error code

Pre-run output with abort:

```json
[
  {
    "__meta": {
      "id": "capture_git_repo",
      "config": {
        "allow_dirty": false
      },
      "success": false,
      "error": "Repository is dirty and allow_dirty=false"
    },
    "commit": "a1b2c3d4...",
    "dirty": true,
    "abort_requested": true
  }
]
```

## Error Handling

### Non-Fatal Errors

- Not in a Git repository (captured as error, execution continues)
- Cannot determine remote URL (captured as null)

```json
{
  "__meta": {
    "id": "capture_git_repo",
    "success": false,
    "error": "Not a git repository"
  }
}
```

### Fatal Errors (Abort)

- Repository is dirty when `allow_dirty = false`

## Git Status Detection

The hook detects:

- **Modified files**: Files with changes in working directory
- **Staged files**: Files added to staging area
- **Untracked files**: New files not tracked by Git
- **Deleted files**: Files removed from working directory

All of these count as "dirty" state.

## See Also

- [capture_file](capture-file.md) - Capture specific files
- [Configuration Guide](../configuration.md) - Error handling details
