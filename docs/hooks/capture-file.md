# capture-file

Captures files matching glob patterns, computes hashes, and copies/moves files to the run directory.

## Configuration

```toml
# Copy file to run directory
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"

# Compute file hash only
[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "none"
hash = "sha256"

# Both copy and hash
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.pkl"
mode = "copy"
hash = "sha256"
```

## Parameters

- `glob` (required): Glob pattern to match files (e.g., "*.txt", "results/**/*.pkl")
- `mode` (optional, default: `"copy"`): File handling mode
- `hash` (optional, default: `"sha256"`): Hash algorithm to use

### File Modes

- `"copy"` - Copy files to run directory (default)
- `"move"` - Move files to run directory
- `"none"` - Don't copy or move, only record metadata

### Hash Algorithms

- `"sha256"` - SHA-256 (default)
- `"none"` - Don't compute hash

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "results/model.pkl",
      "mode": "copy",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "path": "results/model.pkl",
      "copied_path": ".capsula/experiments/2025-12-30/143022-chubby-back/model.pkl",
      "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    }
  ]
}
```

### Fields

- `files` (array): Array of captured file objects
  - `path` (string): Original file path
  - `copied_path` (string, optional): Destination path (if mode is `"copy"` or `"move"`)
  - `hash` (string, optional): Cryptographic hash prefixed with algorithm (e.g., `"sha256:..."`) (if hash is `"sha256"`)

## Use Cases

### Archive Input Configuration

Preserve configuration files used for experiments:

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
```

### Archive Output Files

Save experiment results:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/metrics.json"
mode = "copy"
```

### Verify File Integrity

Compute hashes to verify files haven't changed:

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "data/train.csv"
mode = "none"
hash = "sha256"

[[post-run.hooks]]
id = "capture-file"
glob = "data/train.csv"
mode = "none"
hash = "sha256"
```

### Track Large Files Without Copying

For large model files, compute hash instead of copying:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "models/large_model.bin"
mode = "none"
hash = "sha256"
```

### Capture Multiple Files with Glob Patterns

Use glob patterns to capture multiple files:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.json"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "outputs/**/*.png"
mode = "copy"
```

## Examples

### Preserve Input and Output

```toml
[vault]
name = "training-runs"

# Capture input configuration
[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"

# Capture trained model
[[post-run.hooks]]
id = "capture-file"
glob = "model.pkl"
mode = "copy"
hash = "sha256"
```

### Multiple Output Files

```toml
[vault]
name = "analysis"

[[post-run.hooks]]
id = "capture-file"
glob = "results/*.txt"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "results/*.png"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "results/*.csv"
mode = "copy"
```

### Hash-Only for Large Files

```toml
[vault]
name = "big-data-processing"

# Don't copy large files, just record their hashes
[[post-run.hooks]]
id = "capture-file"
glob = "output/processed_data.parquet"
mode = "none"
hash = "sha256"
```

Output:

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "output/processed_data.parquet",
      "mode": "none",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "path": "output/processed_data.parquet",
      "hash": "sha256:a1b2c3d4e5f6..."
    }
  ]
}
```

### Verify Data Integrity

Check that input data wasn't modified during execution:

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "data/input.csv"
mode = "none"
hash = "sha256"

[[post-run.hooks]]
id = "capture-file"
glob = "data/input.csv"
mode = "none"
hash = "sha256"
```

Compare the hashes in `pre-run.json` and `post-run.json` to verify integrity.

## Glob Patterns

### Basic Patterns

Glob patterns are relative to the project root (where `capsula.toml` is located):

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/output.txt"  # Single file

[[post-run.hooks]]
id = "capture-file"
glob = "*.log"  # All .log files in project root

[[post-run.hooks]]
id = "capture-file"
glob = "results/*.json"  # All .json files in results/
```

### Recursive Patterns

Use `**` for recursive matching:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "**/*.py"  # All .py files recursively

[[post-run.hooks]]
id = "capture-file"
glob = "outputs/**/data.csv"  # All data.csv files under outputs/
```

### File Names in Output

When files are copied, only the filename is preserved (not the directory structure):

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "deep/nested/path/file.txt"
mode = "copy"
```

Copied to: `.capsula/vault/date/time-name/file.txt` (not `deep/nested/path/file.txt`)

## Performance Considerations

### Hash Computation

Computing hashes reads the entire file:

- **Small files (< 10 MB)**: Negligible overhead
- **Large files (> 1 GB)**: Can take several seconds

For large files, consider computing hashes only when necessary.

### File Copying

Copying files:

- Creates a duplicate of the file
- Uses disk space in the `.capsula/` directory
- Fast for small files, slower for large files

For very large output files, consider:

1. Computing hash only (`mode = "none", hash = "sha256"`)
2. Storing files elsewhere and recording their path

## Error Handling

### Common Errors

**File not found**:

```json
{
  "__meta": {
    "id": "capture_file",
    "config": {
      "path": "missing.txt"
    },
    "success": false,
    "error": "File not found: missing.txt"
  }
}
```

**Permission denied**:

```json
{
  "__meta": {
    "id": "capture_file",
    "success": false,
    "error": "Permission denied: protected_file.txt"
  }
}
```

**Copy destination error**:

```json
{
  "__meta": {
    "id": "capture_file",
    "success": false,
    "error": "Failed to copy file: disk full"
  }
}
```

### Non-Fatal Behavior

File capture errors are non-fatal. If a file cannot be captured:

1. Error is logged
2. Error recorded in JSON output
3. Execution continues with remaining hooks

## See Also

- [capture_command](capture-command.md) - Execute commands and capture output
- [capture_git_repo](capture-git-repo.md) - Capture repository state
