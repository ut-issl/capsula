# capture-git-repo

Captures Git repository state including commit hash and dirty status.

## Configuration

```toml
# Allow execution with uncommitted changes
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-repo"
path = "."
allow_dirty = true

# Abort if repository has uncommitted changes
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-repo"
path = "."
allow_dirty = false
```

## Parameters

- `name` (required): Name for this repository (used for patch file naming)
- `path` (required): Path to the repository (relative to project root, or absolute)
- `allow_dirty` (optional, default: `false`): If `false`, aborts the run when the repository has uncommitted changes

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-repo",
      "path": ".",
      "allow_dirty": false
    },
    "success": true
  },
  "working_dir": "/Users/alice/projects/capsula",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": false
}
```

### Fields

- `working_dir` (string): Absolute path to the repository working directory
- `sha` (string): Full commit hash (SHA-1)
- `is_dirty` (boolean): Whether repository has uncommitted changes

If the repository is dirty, a patch file is also created at `.capsula/{vault}/{date}/{time-name}/{name}.patch`

## Use Cases

### Ensure Reproducibility

Require clean repository state for experiments:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "main-repo"
path = "."
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
[[pre-run.hooks]]
id = "capture-git-repo"
name = "main-repo"
path = "."
allow_dirty = true
```

### Detect Code Changes During Execution

Compare repository state before and after:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "main-repo"
path = "."

[[post-run.hooks]]
id = "capture-git-repo"
name = "main-repo"
path = "."
```

### Monitor Multiple Repositories

Track multiple repositories (e.g., code and data):

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "code"
path = "."

[[pre-run.hooks]]
id = "capture-git-repo"
name = "data"
path = "../data-repo"
```

## Examples

### Strict Reproducibility Mode

```toml
[vault]
name = "research-experiments"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "research-repo"
path = "."
allow_dirty = false  # Force clean repository

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"
```

```bash
# This will fail if you have uncommitted changes
capsula run python train_model.py
```

### Relaxed Mode with Tracking

```toml
[vault]
name = "development-runs"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "dev-repo"
path = "."
allow_dirty = true  # Allow uncommitted changes
```

Output when repository is dirty:

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "dev-repo",
      "path": ".",
      "allow_dirty": true
    },
    "success": true
  },
  "working_dir": "/Users/alice/projects/capsula",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": true
}
```

A patch file is also created at `.capsula/development-runs/2025-12-30/143022-chubby-back/dev-repo.patch`.

### Compare Before and After

```toml
[vault]
name = "code-generation"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "codegen-repo"
path = "."
allow_dirty = true

[[post-run.hooks]]
id = "capture-git-repo"
name = "codegen-repo"
path = "."
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
      "id": "capture-git-repo",
      "config": {
        "name": "my-repo",
        "path": ".",
        "allow_dirty": false
      },
      "success": true
    },
    "working_dir": "/Users/alice/projects/capsula",
    "sha": "a1b2c3d4...",
    "is_dirty": true
  }
]
```

The run will be aborted after pre-run hooks complete because `allow_dirty = false` and the repository is dirty.

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
