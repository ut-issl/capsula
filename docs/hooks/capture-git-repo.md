# capture-git-repo

Captures git repository state including commit hash, branch, and whether there are uncommitted changes.

## Use Cases

- **Ensure reproducibility** - Record the exact code version used
- **Prevent dirty runs** - Abort execution if there are uncommitted changes
- **Track experiment versions** - Know which code produced which results
- **Audit compliance** - Prove which code version was used

## Configuration

### Required Options

| Option | Type | Description |
|--------|------|-------------|
| `path` | string | Path to the git repository (`.` for current directory) |

### Optional Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allow_dirty` | boolean | `false` | If `false`, Capsula aborts when the repository has uncommitted changes |

### Example

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

## Output Example

### Clean Repository

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "path": ".",
      "allow_dirty": false
    },
    "success": true
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": false,
  "abort_on_dirty": false
}
```

### Dirty Repository (with `allow_dirty = true`)

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "path": ".",
      "allow_dirty": true
    },
    "success": true
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": true,
  "abort_on_dirty": false
}
```

### Dirty Repository (with `allow_dirty = false`)

When `allow_dirty = false` and the repository is dirty, Capsula:

1. Saves the hook output showing the dirty state
2. **Aborts before running your command**
3. Exits with an error message

```
Error: Hook 'capture-git-repo' requested abort: Repository has uncommitted changes
```

### Output Fields

| Field | Type | Description |
|-------|------|-------------|
| `working_dir` | string | Absolute path to the repository |
| `sha` | string | Full commit hash (SHA-1) |
| `is_dirty` | boolean | Whether there are uncommitted changes |
| `abort_on_dirty` | boolean | Whether abort was requested due to dirty state |

## Complete Example

### Strict Mode (Abort on Dirty)

For reproducible experiments:

```toml title="capsula.toml"
[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Abort if repo is dirty
```

Run:

```bash
# If you have uncommitted changes:
capsula run python train.py
# Error: Hook 'capture-git-repo' requested abort: Repository has uncommitted changes

# Commit your changes first:
git add .
git commit -m "Ready to run experiment"
capsula run python train.py
# ✓ Runs successfully
```

### Permissive Mode (Allow Dirty)

For development and testing:

```toml title="capsula.toml"
[vault]
name = "dev-runs"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true  # Allow uncommitted changes
```

Run:

```bash
# Even with uncommitted changes:
capsula run python test.py
# ✓ Runs successfully (dirty state is recorded)
```

## What Counts as "Dirty"?

A repository is considered dirty if it has:

- **Modified tracked files** - Files that have been edited
- **Staged changes** - Changes added with `git add`
- **Untracked files** - New files not in `.gitignore`

Files in `.gitignore` are **not** considered - they don't make the repository dirty.

### Examples

```bash
# Clean repository
git status
# nothing to commit, working tree clean

# Dirty: modified file
echo "new line" >> file.txt
git status
# modified: file.txt

# Dirty: untracked file
echo "test" > new_file.txt
git status
# Untracked files: new_file.txt

# Not dirty: ignored file
echo "temp" > .gitignore-file
# (if .gitignore-file is in .gitignore)
```

## Tips

### Use Strict Mode for Production

For reproducible experiments and production runs, always use `allow_dirty = false`:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

This ensures you can always reproduce results from the exact code that was run.

### Use Permissive Mode for Development

For quick testing and development, use `allow_dirty = true`:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true
```

This lets you iterate quickly without committing every change.

### Capture Multiple Repositories

If your project uses multiple repositories, capture them all:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false

[[pre-run.hooks]]
id = "capture-git-repo"
path = "../dependency-repo"
allow_dirty = false
```

### Check Git State in CI/CD

In CI/CD pipelines, the repository is typically clean. Verify this:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

## Common Questions

**Q: What if I'm not in a git repository?**

The hook will fail with an error:

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "success": false,
    "error": "Not a git repository"
  }
}
```

The error is non-fatal - Capsula will continue with remaining hooks.

**Q: Can I capture branch names or tags?**

Currently, only the commit hash (`sha`) is captured. The commit hash is sufficient to identify the exact code version. You can find the branch/tag later using:

```bash
git branch --contains <sha>
git tag --contains <sha>
```

**Q: What if my repository is in a detached HEAD state?**

The commit hash is still captured correctly. Detached HEAD is not considered an error.

**Q: Will this abort if I have untracked files?**

Yes, if `allow_dirty = false`. Untracked files (that aren't in `.gitignore`) make the repository dirty.

To avoid this, either:

1. Add files to `.gitignore`
2. Commit or delete the files
3. Use `allow_dirty = true`

**Q: Can I see what changes made the repository dirty?**

The hook doesn't capture the diff. However, you know:

- The commit hash that was checked out
- That there were uncommitted changes

If you want to save the diff, use:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["git", "diff"]
```

**Q: Does this work with git submodules?**

The hook captures the state of the specified repository only. Submodules are treated as regular directories and their uncommitted changes may make the parent repository dirty.

To capture submodules separately:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-git-repo"
path = "submodules/my-submodule"
```

## Workflow Example

### Research Experiment Workflow

1. **Make changes and test**

```bash
# Development mode: allow dirty
capsula --config dev.toml run python test.py
```

```toml title="dev.toml"
[vault]
name = "dev"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true
```

2. **Commit when ready**

```bash
git add .
git commit -m "Ready for experiment"
```

3. **Run experiment with strict mode**

```bash
# Production mode: require clean repo
capsula --config experiment.toml run python train.py
```

```toml title="experiment.toml"
[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false
```

## Related Hooks

- [capture-cwd](capture-cwd.md) - Capture working directory
- [capture-command](capture-command.md) - Run `git` commands to capture more details

[:octicons-arrow-left-24: Back to Hooks](../hooks.md)
