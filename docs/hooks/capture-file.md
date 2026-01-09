# capture-file

Captures files by copying them, moving them, or computing their hash.

## Use Cases

- **Preserve input configurations** - Save config files used for experiments
- **Archive output results** - Save generated data, models, and plots
- **Verify file integrity** - Compute hashes without copying large files
- **Organize experiment artifacts** - Automatically collect outputs in one place

## Configuration

### Required Options

| Option | Type | Description |
|--------|------|-------------|
| `glob` | string | File pattern to match (e.g., `"*.txt"`, `"results/**/*.png"`) |

### Optional Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `mode` | string | `"copy"` | How to handle files: `"copy"`, `"move"`, or `"none"` |
| `hash` | string | `"sha256"` | Hash algorithm: `"sha256"` or `"none"` |

### Example

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.csv"
mode = "copy"
hash = "sha256"
```

## File Modes

### `mode = "copy"` (default)

Copies files to the run directory, leaving originals in place.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
```

**Use when:** You want to preserve files while keeping them in their original location.

### `mode = "move"`

Moves files to the run directory, removing them from their original location.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "move"
```

**Use when:** You want to archive outputs and don't need them in the original location.

!!! warning "Files are removed"
    When using `mode = "move"`, files are deleted from their original location. Make sure you won't need them there!

### `mode = "none"`

Doesn't copy or move files, only computes their hash (if `hash` is enabled).

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "large-model.bin"
mode = "none"
hash = "sha256"
```

**Use when:** You want to verify file integrity without duplicating large files.

## Hash Options

### `hash = "sha256"` (default)

Computes SHA-256 hash of each file.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "data.csv"
hash = "sha256"
```

**Use when:** You want to verify file integrity or detect changes.

### `hash = "none"`

Skips hash computation.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "*.txt"
mode = "copy"
hash = "none"
```

**Use when:** You don't need hash verification (slightly faster).

## Glob Patterns

Glob patterns follow standard glob syntax:

| Pattern | Matches |
|---------|---------|
| `*` | Any filename (not crossing directories) |
| `**` | Any path including subdirectories |
| `?` | Single character |
| `[abc]` | Any character in set |

### Examples

```toml
# All .txt files in current directory
glob = "*.txt"

# All .csv files in results/ and subdirectories
glob = "results/**/*.csv"

# Specific numbered files
glob = "data_?.json"  # data_1.json, data_2.json, etc.

# Multiple extensions
glob = "output.{txt,log,csv}"

# All files in a directory
glob = "outputs/*"

# All files recursively
glob = "outputs/**/*"
```

## Output Example

### With Copy Mode

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "*.txt",
      "mode": "copy",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "path": "/path/to/project/output.txt",
      "copied_path": "/path/to/.capsula/vault/2025-01-09/143022-happy-river/output.txt",
      "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    {
      "path": "/path/to/project/results.txt",
      "copied_path": "/path/to/.capsula/vault/2025-01-09/143022-happy-river/results.txt",
      "hash": "sha256:d4735e3a265e16eee03f59718b9b5d03019c07d8b6c51f90da3a666eec13ab35"
    }
  ]
}
```

### With None Mode

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "model.bin",
      "mode": "none",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "path": "/path/to/project/model.bin",
      "hash": "sha256:a3c7b9..."
    }
  ]
}
```

### Output Fields

| Field | Type | Description |
|-------|------|-------------|
| `files` | array | List of matched files |
| `files[].path` | string | Original absolute path to the file |
| `files[].copied_path` | string | Path where file was copied/moved (only for `copy` or `move` mode) |
| `files[].hash` | string | SHA-256 hash of the file (only if `hash = "sha256"`) |

## Complete Examples

### Capture Input Configuration

```toml title="capsula.toml"
[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"
```

Result:

```
.capsula/experiments/2025-01-09/143022-happy-river/
├── _capsula/
│   └── pre-run.json
└── config.yaml  # Copied from project root
```

### Archive All Results

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/**/*"
mode = "move"
```

Result: All files from `results/` are moved to the run directory.

### Verify Large File Integrity

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "model.bin"
mode = "none"
hash = "sha256"
```

Result: Hash is computed and saved, but file stays in place.

### Multiple File Patterns

```toml
# Capture CSVs
[[post-run.hooks]]
id = "capture-file"
glob = "*.csv"
mode = "copy"

# Capture PNGs
[[post-run.hooks]]
id = "capture-file"
glob = "plots/*.png"
mode = "copy"

# Capture model (hash only)
[[post-run.hooks]]
id = "capture-file"
glob = "model.pkl"
mode = "none"
hash = "sha256"
```

## Tips

### Capture Inputs in Pre-Run

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "config.json"
mode = "copy"
```

### Capture Outputs in Post-Run

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*"
mode = "copy"
```

### Use Copy for Shared Files

If multiple hooks need the same file, use `copy` not `move`:

```toml
# File can be used by Slack hook
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]
```

### Hash Large Files Without Copying

For very large files, compute hash without copying:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "dataset.tar.gz"
mode = "none"
hash = "sha256"
```

### Organize by File Type

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "*.log"
mode = "move"

[[post-run.hooks]]
id = "capture-file"
glob = "*.csv"
mode = "move"

[[post-run.hooks]]
id = "capture-file"
glob = "*.png"
mode = "move"
```

## Common Patterns

### Pattern: Save Configuration

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "*.{yaml,yml,json,toml}"
mode = "copy"
```

### Pattern: Archive Experiment Results

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/**/*"
mode = "move"
```

### Pattern: Save Plots

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "**/*.{png,jpg,pdf,svg}"
mode = "copy"
```

### Pattern: Verify Checksums

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "data/*.dat"
mode = "none"
hash = "sha256"
```

## Common Questions

**Q: What if no files match the glob pattern?**

The hook succeeds with an empty files array:

```json
{
  "__meta": {
    "id": "capture-file",
    "success": true
  },
  "files": []
}
```

**Q: What if a file is too large?**

There's no size limit, but copying very large files may take time. For files larger than a few GB, consider using `mode = "none"` with `hash = "sha256"` to just verify integrity.

**Q: Can I exclude files?**

Not directly, but you can use more specific glob patterns:

```toml
# Exclude backup files
glob = "results/[!~]*.csv"  # Doesn't match files starting with ~
```

Or use multiple hooks with specific patterns.

**Q: What if a file disappears between matching and copying?**

The hook will fail for that file and log an error, but continue with remaining files.

**Q: Does `move` work across filesystems?**

Yes, Capsula handles cross-filesystem moves by copying and then deleting.

**Q: Can I capture files from outside the project directory?**

Glob patterns are resolved relative to the project root (where `capsula.toml` is). You can use `../` to go up:

```toml
glob = "../shared-data/*.csv"
```

However, be careful with absolute paths - globs should be relative paths.

**Q: What about symbolic links?**

Symbolic links are followed and the actual files are copied/hashed.

**Q: Will hidden files be matched?**

Yes, glob patterns match hidden files (starting with `.`):

```toml
glob = ".*"  # Matches .env, .gitignore, etc.
```

## Hook Order Considerations

### ⚠️ Move Before Attach

If using Slack attachments, `capture-file` with `mode = "move"` must come **after** the Slack hook:

```toml
# BAD: File is moved before Slack can attach it
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]  # File already moved!
```

```toml
# GOOD: Slack attaches file, then it's moved
[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]

[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"
```

Or use `mode = "copy"`:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "copy"  # File stays available

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]
```

## Related Hooks

- [capture-git-repo](capture-git-repo.md) - Capture source code version
- [notify-slack](notify-slack.md) - Send files to Slack

[:octicons-arrow-left-24: Back to Configuration](../configuration.md#available-hook-types)
