---
icon: material/hook
---

# capture-env

Captures the value of environment variables.

## Use Cases

- Record which executables were available (PATH)
- Save environment-based configuration (e.g., `CUDA_VISIBLE_DEVICES`)
- Debug environment issues
- Track user context (e.g., `USER`, `HOME`)

## Configuration

### Required Options

| Option | Type | Description |
| -------- | ------ | ------------- |
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
