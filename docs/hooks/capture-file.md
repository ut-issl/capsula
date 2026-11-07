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

### Glob Patterns

`glob` is matched against paths relative to the project root (the directory
containing `capsula.toml`):

- `*` and `?` match within one path component, so `*.txt` matches only files
  at the project root, and `results/*.csv` only files directly in `results/`.
- `**` matches any number of directories, so `**/*.txt` matches `.txt` files
  at any depth.
- Use `/` as the path separator. On Windows, `\` is also a separator; on other
  platforms it is an ordinary file name character.
- A pattern that is absolute (such as `/home/me/.aws/*`) or contains a `..`
  component (such as `../data/*`) is rejected when the configuration is
  loaded.
- Symbolic links are followed, as in a shell, so `data/*.csv` matches files
  in `data/` even if `data` is a link to a directory elsewhere.

Only directories that can contain a match are read: `results/*.csv` reads
`results/` but no other directory, while `**` reads every directory below
it. A directory that must be read but cannot be fails the hook.

!!! note "This is not a security boundary"
    Rejecting absolute and `..` patterns prevents mistakes, such as moving
    files out of another project. It does not stop a malicious
    `capsula.toml`, which could capture the same files through a symbolic
    link or another hook such as `capture-command`. Run only configurations
    you trust.

### Mode Options

- `"copy"` - Copies files to the hook's artifact directory, leaves originals
- `"move"` - Moves files to the hook's artifact directory, removes originals
- `"none"` - Only computes hash, doesn't copy or move

!!! note "Per-hook artifact directory"
    When `mode` is `"copy"` or `"move"`, files are placed in a per-hook
    artifact directory named `{phase}-{index}-capture-file/` under the run
    directory (e.g., `post-0-capture-file/`). This prevents filename
    collisions between different `capture-file` hooks.

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
      "src": "results/data.csv",
      "dst": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/data.csv",
      "hash": "a1b2c3d4e5f6...",
      "copied": true
    },
    {
      "src": "results/summary.csv",
      "dst": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/summary.csv",
      "hash": "b2c3d4e5f6g7...",
      "copied": true
    }
  ]
}
```
