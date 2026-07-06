---
icon: material/hook
---

# capture-dir

Captures a directory tree by copying it, moving it, or recording file hashes.

## Use Cases

- Preserve a full output directory
- Archive generated reports with nested assets
- Hash all files in an input dataset without copying it

## Configuration

### Required Options

| Option | Type | Description |
| -------- | ------ | ------------- |
| `path` | string | Directory path to capture, relative to the project root unless absolute |

### Optional Options

| Option | Type | Default | Description |
| -------- | ------ | --------- | ------------- |
| `mode` | string | `"copy"` | How to handle the directory: `"copy"`, `"move"`, or `"none"` |
| `hash` | string | `"sha256"` | Hash algorithm for files: `"sha256"` or `"none"` |

### Mode Options

- `"copy"` - Copies the directory into the hook's artifact directory and leaves the original directory in place
- `"move"` - Moves the directory into the hook's artifact directory and removes the original directory
- `"none"` - Only records matching file metadata and hashes; no artifact directory is created

!!! note "Per-hook artifact directory"
    For `"copy"` and `"move"`, the captured directory is placed under a
    per-hook artifact directory named `{phase}-{index}-capture-dir/` under the
    run directory (for example, `post-0-capture-dir/results/`). The directory
    tree is preserved.

### Example

```toml
[[post-run.hooks]]
id = "capture-dir"
path = "results"
mode = "copy"
hash = "sha256"
```

## Output Example

```json
{
  "__meta": {
    "id": "capture-dir",
    "config": {
      "path": "results",
      "mode": "copy",
      "hash": "sha256"
    },
    "success": true
  },
  "path": "/path/to/project/results",
  "captured_path": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-dir/results",
  "directories": ["assets"],
  "files": [
    {
      "path": "/path/to/project/results/data.csv",
      "relative_path": "data.csv",
      "captured_path": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-dir/results/data.csv",
      "hash": "sha256:a1b2c3d4e5f6..."
    },
    {
      "path": "/path/to/project/results/assets/plot.png",
      "relative_path": "assets/plot.png",
      "captured_path": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-dir/results/assets/plot.png",
      "hash": "sha256:b2c3d4e5f6g7..."
    }
  ]
}
```
