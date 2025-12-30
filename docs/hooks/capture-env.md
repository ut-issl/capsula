# capture-env

Captures a single environment variable.

## Configuration

```toml
# Capture a specific environment variable
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

# Capture multiple variables (use multiple hook instances)
[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"

[[pre-run.hooks]]
id = "capture-env"
name = "HOME"
```

## Parameters

- `name` (required): Name of the environment variable to capture

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture-env",
    "config": {
      "name": "PATH"
    },
    "success": true
  },
  "value": "/usr/local/bin:/usr/bin:/bin"
}
```

### Fields

- `value` (string or null): Value of the environment variable, or `null` if not set

## Use Cases

### Capture Python Environment

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"

[[pre-run.hooks]]
id = "capture-env"
name = "VIRTUAL_ENV"

[[pre-run.hooks]]
id = "capture-env"
name = "CONDA_DEFAULT_ENV"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHON_VERSION"
```

### Capture CUDA Configuration

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_HOME"

[[pre-run.hooks]]
id = "capture-env"
name = "LD_LIBRARY_PATH"
```

### Compare Environment Changes

Capture environment variables before and after to detect changes:

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[post-run.hooks]]
id = "capture-env"
name = "PATH"
```

## Examples

### Minimal Example

```toml
[vault]
name = "env-tracking"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "USER"

[[pre-run.hooks]]
id = "capture-env"
name = "HOME"
```

```bash
capsula run echo "Environment captured"
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture-env",
      "config": {
        "name": "PATH"
      },
      "success": true
    },
    "value": "/usr/local/bin:/usr/bin:/bin"
  },
  {
    "__meta": {
      "id": "capture-env",
      "config": {
        "name": "USER"
      },
      "success": true
    },
    "value": "alice"
  },
  {
    "__meta": {
      "id": "capture-env",
      "config": {
        "name": "HOME"
      },
      "success": true
    },
    "value": "/Users/alice"
  }
]
```

## Security Considerations

!!! warning "Sensitive Data"
    Be careful when capturing environment variables. Many applications store secrets in environment variables. Only capture variables you need and avoid capturing sensitive data.

Variables to avoid capturing:

- API keys and tokens (e.g., `GITHUB_TOKEN`, `API_KEY`)
- Passwords
- Webhook URLs (e.g., `SLACK_WEBHOOK_URL`)
- AWS credentials (e.g., `AWS_SECRET_ACCESS_KEY`)
- SSH keys or passphrases

## Error Handling

This hook rarely fails. If the environment variable doesn't exist, the `value` field will be `null`.

## See Also

- [capture_cwd](capture-cwd.md) - Capture working directory
- [capture_machine](capture-machine.md) - Capture system information
