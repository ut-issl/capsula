# capture-git-repo

Captures git repository state including commit hash, branch, and whether there are uncommitted changes.

## Use Cases

- Record the exact code version used for reproducibility
- Prevent execution if there are uncommitted changes
- Track which code produced which results
- Audit which code version was used

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

!!! warning "Abort Behavior"
    When `allow_dirty = false` and the repository is dirty, Capsula saves the hook output showing the dirty state, then aborts before running your command.
