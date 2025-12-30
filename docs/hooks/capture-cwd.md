# capture_cwd

Captures the current working directory.

## Configuration

```toml
[[pre_run]]
type = "capture_cwd"
```

## Parameters

This hook has no configuration parameters.

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture_cwd",
    "config": {},
    "success": true
  },
  "cwd": "/Users/alice/projects/my-project"
}
```

### Fields

- `cwd` (string): Absolute path to the current working directory

## Use Cases

### Document Execution Location

Capture where your command was executed:

```toml
[[pre_run]]
type = "capture_cwd"
```

### Track Directory Changes

Compare working directory before and after execution:

```toml
[[pre_run]]
type = "capture_cwd"

[[post_run]]
type = "capture_cwd"
```

## Examples

### Basic Usage

```toml
[vault]
name = "my-experiments"

[[pre_run]]
type = "capture_cwd"
```

```bash
capsula run python script.py
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture_cwd",
      "config": {},
      "success": true
    },
    "cwd": "/Users/alice/projects/ml-project"
  }
]
```

## Error Handling

This hook rarely fails. Possible error:

- Unable to determine current directory (e.g., directory was deleted)

Error output:

```json
{
  "__meta": {
    "id": "capture_cwd",
    "config": {},
    "success": false,
    "error": "Failed to get current directory: No such file or directory"
  }
}
```

## See Also

- [capture_env](capture-env.md) - Capture environment variables
- [capture_machine](capture-machine.md) - Capture system information
