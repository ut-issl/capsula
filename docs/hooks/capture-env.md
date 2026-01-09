# capture-env

Captures the value of environment variables.

## Use Cases

- **Record PATH** - Know which executables were available
- **Capture tool settings** - Save environment-based configuration (e.g., `CUDA_VISIBLE_DEVICES`, `OMP_NUM_THREADS`)
- **Debug environment issues** - Understand what environment variables were set
- **Audit user context** - Record who ran the command (e.g., `USER`, `HOME`)

## Configuration

### Required Options

| Option | Type | Description |
|--------|------|-------------|
| `name` | string | Name of the environment variable to capture |

### Example

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"
```

## Output Example

### When Variable Exists

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

### When Variable Doesn't Exist

```json
{
  "__meta": {
    "id": "capture-env",
    "config": {
      "name": "MY_VAR"
    },
    "success": true
  },
  "value": null
}
```

!!! info
    A `null` value means the variable is not set. This is still considered a successful hook execution.

### Output Fields

| Field | Type | Description |
|-------|------|-------------|
| `value` | string or null | Value of the environment variable, or `null` if not set |

## Complete Example

```toml title="capsula.toml"
[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-env"
name = "USER"
```

Run:

```bash
export CUDA_VISIBLE_DEVICES=0,1
capsula run python train.py
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture-env",
      "config": {"name": "PATH"},
      "success": true
    },
    "value": "/usr/local/bin:/usr/bin:/bin"
  },
  {
    "__meta": {
      "id": "capture-env",
      "config": {"name": "PYTHONPATH"},
      "success": true
    },
    "value": "/home/user/lib"
  },
  {
    "__meta": {
      "id": "capture-env",
      "config": {"name": "CUDA_VISIBLE_DEVICES"},
      "success": true
    },
    "value": "0,1"
  },
  {
    "__meta": {
      "id": "capture-env",
      "config": {"name": "USER"},
      "success": true
    },
    "value": "nomura"
  }
]
```

## Common Patterns

### Capture Multiple Variables

Use multiple hook declarations to capture several variables:

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "HOME"

[[pre-run.hooks]]
id = "capture-env"
name = "USER"
```

### Capture ML/GPU Settings

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

### Capture Python Settings

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PYTHONPATH"

[[pre-run.hooks]]
id = "capture-env"
name = "VIRTUAL_ENV"

[[pre-run.hooks]]
id = "capture-env"
name = "PYTHON_VERSION"
```

### Capture Build Settings

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "CC"

[[pre-run.hooks]]
id = "capture-env"
name = "CXX"

[[pre-run.hooks]]
id = "capture-env"
name = "CFLAGS"
```

## Tips

### Check for Missing Variables

If you expect a variable to be set, review the output to verify it's not `null`:

```bash
# Check if CUDA_VISIBLE_DEVICES was captured
cat .capsula/my-vault/*/latest/_capsula/pre-run.json | grep CUDA_VISIBLE_DEVICES
```

### Use with dotenv

Combine with the `dotenv` option to load variables from a file:

```toml
dotenv = ".env"

[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-env"
name = "API_KEY"  # Loaded from .env
```

### Capture Capsula Variables

You can also capture Capsula's own environment variables (though they're not set during hook execution):

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "echo $CAPSULA_RUN_ID"]
```

## Common Questions

**Q: Can I capture all environment variables at once?**

No, you need to specify each variable individually. This is by design - it ensures you explicitly choose what to capture rather than accidentally saving sensitive data.

**Q: What if my variable contains sensitive data?**

Be careful! Environment variables are saved in plain text JSON. Don't capture variables containing:

- Passwords
- API keys
- Tokens
- Other secrets

Instead, reference them by name in documentation rather than capturing their values.

**Q: Can I capture variables that don't exist yet?**

Yes - the hook will record a `null` value. This can be useful for documentation:

```toml
# Document expected variables even if not set
[[pre-run.hooks]]
id = "capture-env"
name = "OPTIONAL_CONFIG"
```

**Q: Does this affect my command's environment?**

No, this hook only reads environment variables - it doesn't modify them. Your command sees the same environment whether or not you use this hook.

## Related Hooks

- [capture-cwd](capture-cwd.md) - Capture working directory (similar to `PWD` env var)
- [capture-command](capture-command.md) - Run commands that use environment variables

[:octicons-arrow-left-24: Back to Hooks](../hooks.md)
