---
icon: material/hook
---

# capture-file

Captures files by copying them, moving them, or computing their hash.

## Use Cases

- Preserve input configurations
- Archive output results
- Verify file integrity without copying large files
- Organize experiment artifacts

## Configuration

### Required Options

| Option | Type | Description |
| -------- | ------ | ------------- |
| `glob` | string | File pattern to match (e.g., `"*.txt"`, `"results/**/*.png"`) |

### Optional Options

| Option | Type | Default | Description |
| -------- | ------ | --------- | ------------- |
| `mode` | string | `"copy"` | How to handle files: `"copy"`, `"move"`, or `"none"` |
| `hash` | string | `"sha256"` | Hash algorithm: `"sha256"` or `"none"` |

### Glob Path Rules

Glob patterns are always relative to the project root (the directory containing
`capsula.toml`). Absolute patterns and patterns containing a `..` path component
are rejected.

- `*.txt` matches files at the project root.
- `results/*.csv` matches files directly under `results/`.
- `**/*.txt` matches files recursively.
- Use `/` as the portable path separator. On Unix, `\` remains a legal,
  literal filename character.

Symbolic links are not followed. A symbolic link that directly matches the glob
causes the hook to fail, and symbolic-link directories are not traversed.

!!! warning "Project containment"
    A `capture-file` hook cannot read, hash, copy, or move files outside the
    project root. Use an explicit project-local path instead of an absolute or
    parent-relative path.

### Mode Options

- `"copy"` - Copies files to the hook's artifact directory, leaves originals
- `"move"` - Copies files to the hook's artifact directory, then removes the originals
- `"none"` - Only computes hashes, without copying or moving files

When hashing is enabled for `"copy"` or `"move"`, each hash is computed from
the completed artifact rather than reading the source again. In `"move"` mode,
the source is removed only after the artifact has been successfully copied and
any configured hash has been computed. In `"none"` mode, the source itself is
hashed.

!!! note "Per-hook artifact directory"
    When `mode` is `"copy"` or `"move"`, files are placed in a per-hook
    artifact directory named `{phase}-{index}-capture-file/` under the run
    directory (e.g., `post-0-capture-file/`). Each file retains its path
    relative to the project root: `results/a/data.csv` is stored as
    `post-0-capture-file/results/a/data.csv`. Existing destinations are rejected
    rather than overwritten.

### Example

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.csv"
mode = "copy"
hash = "sha256"
```

## Output Example

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "results/*.csv",
      "mode": "copy",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "path": "/project/results/data.csv",
      "copied_path": "/project/.capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/results/data.csv",
      "hash": "sha256:a1b2c3d4e5f6..."
    },
    {
      "path": "/project/results/summary.csv",
      "copied_path": "/project/.capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/results/summary.csv",
      "hash": "sha256:b2c3d4e5f6g7..."
    }
  ]
}
```
