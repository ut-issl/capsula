# capture_file

Captures file content, computes hashes, and copies files to the run directory.

## Configuration

```toml
# Copy file to run directory
[[post_run]]
type = "capture_file"
path = "output.txt"
copy = true

# Compute file hash
[[pre_run]]
type = "capture_file"
path = "config.yaml"
compute_hash = true
algorithm = "sha256"

# Both copy and hash
[[post_run]]
type = "capture_file"
path = "model.pkl"
copy = true
compute_hash = true
algorithm = "md5"
```

## Parameters

- `path` (required): File path relative to project root
- `copy` (optional, default: `false`): Copy the file to the run directory
- `compute_hash` (optional, default: `false`): Compute cryptographic hash of the file
- `algorithm` (optional, default: `"sha256"`): Hash algorithm to use

### Hash Algorithms

- `"sha256"` - SHA-256 (recommended, default)
- `"sha1"` - SHA-1
- `"md5"` - MD5

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture_file",
    "config": {
      "path": "results/model.pkl",
      "copy": true,
      "compute_hash": true,
      "algorithm": "sha256"
    },
    "success": true
  },
  "path": "results/model.pkl",
  "size": 1048576,
  "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "algorithm": "sha256",
  "copied_to": ".capsula/experiments/2025-12-30/143022-chubby-back/model.pkl"
}
```

### Fields

- `path` (string): Original file path
- `size` (number): File size in bytes
- `hash` (string, optional): Cryptographic hash (if `compute_hash = true`)
- `algorithm` (string, optional): Hash algorithm used (if `compute_hash = true`)
- `copied_to` (string, optional): Destination path (if `copy = true`)

## Use Cases

### Archive Input Configuration

Preserve configuration files used for experiments:

```toml
[[pre_run]]
type = "capture_file"
path = "config.yaml"
copy = true
```

### Archive Output Files

Save experiment results:

```toml
[[post_run]]
type = "capture_file"
path = "results/metrics.json"
copy = true
```

### Verify File Integrity

Compute hashes to verify files haven't changed:

```toml
[[pre_run]]
type = "capture_file"
path = "data/train.csv"
compute_hash = true
algorithm = "sha256"

[[post_run]]
type = "capture_file"
path = "data/train.csv"
compute_hash = true
algorithm = "sha256"
```

### Track Large Files Without Copying

For large model files, compute hash instead of copying:

```toml
[[post_run]]
type = "capture_file"
path = "models/large_model.bin"
compute_hash = true
copy = false  # Don't copy, just hash
```

## Examples

### Preserve Input and Output

```toml
[vault]
name = "training-runs"

# Capture input configuration
[[pre_run]]
type = "capture_file"
path = "config.yaml"
copy = true
compute_hash = true

# Capture trained model
[[post_run]]
type = "capture_file"
path = "model.pkl"
copy = true
compute_hash = true
```

### Multiple Output Files

```toml
[vault]
name = "analysis"

[[post_run]]
type = "capture_file"
path = "results/summary.txt"
copy = true

[[post_run]]
type = "capture_file"
path = "results/plots.png"
copy = true

[[post_run]]
type = "capture_file"
path = "results/data.csv"
copy = true
```

### Hash-Only for Large Files

```toml
[vault]
name = "big-data-processing"

# Don't copy large files, just record their hashes
[[post_run]]
type = "capture_file"
path = "output/processed_data.parquet"
compute_hash = true
copy = false
```

Output:

```json
{
  "__meta": {
    "id": "capture_file",
    "config": {
      "path": "output/processed_data.parquet",
      "compute_hash": true,
      "copy": false,
      "algorithm": "sha256"
    },
    "success": true
  },
  "path": "output/processed_data.parquet",
  "size": 524288000,
  "hash": "a1b2c3d4e5f6...",
  "algorithm": "sha256"
}
```

### Verify Data Integrity

Check that input data wasn't modified during execution:

```toml
[[pre_run]]
type = "capture_file"
path = "data/input.csv"
compute_hash = true

[[post_run]]
type = "capture_file"
path = "data/input.csv"
compute_hash = true
```

Compare the hashes in `pre-run.json` and `post-run.json` to verify integrity.

## File Paths

### Relative Paths

Paths are relative to the project root (where `capsula.toml` is located):

```toml
[[post_run]]
type = "capture_file"
path = "results/output.txt"  # ./results/output.txt
```

### Nested Directories

Files in nested directories are preserved when copied:

```toml
[[post_run]]
type = "capture_file"
path = "deep/nested/path/file.txt"
copy = true
```

Copied to: `.capsula/vault/date/time-name/deep/nested/path/file.txt`

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

1. Computing hash only (`copy = false, compute_hash = true`)
2. Using symbolic links (not currently supported)
3. Storing files elsewhere and recording their path

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
