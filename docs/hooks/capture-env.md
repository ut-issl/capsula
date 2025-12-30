# capture_env

Captures environment variables.

## Configuration

```toml
# Capture all environment variables
[[pre_run]]
type = "capture_env"

# Capture specific variables
[[pre_run]]
type = "capture_env"
include = ["PATH", "PYTHONPATH", "HOME"]

# Exclude specific variables
[[pre_run]]
type = "capture_env"
exclude = ["SECRET_KEY", "API_TOKEN"]

# Combine include and exclude
[[pre_run]]
type = "capture_env"
include = ["PYTHON*"]
exclude = ["PYTHONHASHSEED"]
```

## Parameters

- `include` (optional): Array of environment variable names to capture. If not specified, captures all variables.
- `exclude` (optional): Array of environment variable names to exclude from capture.

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture_env",
    "config": {
      "include": ["PATH", "HOME"]
    },
    "success": true
  },
  "env": {
    "PATH": "/usr/local/bin:/usr/bin:/bin",
    "HOME": "/Users/alice"
  }
}
```

### Fields

- `env` (object): Dictionary of environment variable names and their values

## Use Cases

### Capture Python Environment

```toml
[[pre_run]]
type = "capture_env"
include = [
    "PYTHONPATH",
    "VIRTUAL_ENV",
    "CONDA_DEFAULT_ENV",
    "PYTHON_VERSION"
]
```

### Capture CUDA Configuration

```toml
[[pre_run]]
type = "capture_env"
include = [
    "CUDA_VISIBLE_DEVICES",
    "CUDA_HOME",
    "LD_LIBRARY_PATH"
]
```

### Capture All Except Secrets

```toml
[[pre_run]]
type = "capture_env"
exclude = [
    "AWS_SECRET_ACCESS_KEY",
    "GITHUB_TOKEN",
    "API_KEY",
    "PASSWORD"
]
```

### Compare Environment Changes

Capture environment before and after to detect changes:

```toml
[[pre_run]]
type = "capture_env"

[[post_run]]
type = "capture_env"
```

## Examples

### Minimal Example

```toml
[vault]
name = "env-tracking"

[[pre_run]]
type = "capture_env"
include = ["PATH", "USER", "HOME"]
```

```bash
capsula run echo "Environment captured"
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture_env",
      "config": {
        "include": ["PATH", "USER", "HOME"]
      },
      "success": true
    },
    "env": {
      "PATH": "/usr/local/bin:/usr/bin:/bin",
      "USER": "alice",
      "HOME": "/Users/alice"
    }
  }
]
```

### Exclude Sensitive Variables

```toml
[vault]
name = "safe-env-capture"

[[pre_run]]
type = "capture_env"
exclude = [
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "GITHUB_TOKEN",
    "SLACK_WEBHOOK_URL"
]
```

## Security Considerations

!!! warning "Sensitive Data"
    Be careful when capturing all environment variables. Many applications store secrets in environment variables. Always use `exclude` to prevent capturing sensitive data.

Recommended exclusions:

- API keys and tokens
- Passwords
- Webhook URLs
- AWS credentials
- SSH keys or passphrases

## Error Handling

This hook rarely fails. Possible errors:

- Environment variable specified in `include` doesn't exist (non-fatal, variable is skipped)

## See Also

- [capture_cwd](capture-cwd.md) - Capture working directory
- [capture_machine](capture-machine.md) - Capture system information
