---
icon: material/hook
---

# capture-command

Runs a shell command and captures its output and exit code.

## Use Cases

- Capture tool versions (e.g., Python, Node.js)
- Run diagnostic commands (e.g., `nvidia-smi`, `df`)
- Execute analysis scripts
- Validate preconditions before running main command

## Configuration

### Required Options

| Option | Type | Description |
| -------- | ------ | ------------- |
| `command` | array | Command and arguments as an array (e.g., `["python", "--version"]`) |

### Optional Options

| Option | Type | Default | Description |
| -------- | ------ | --------- | ------------- |
| `success_codes` | array of integers | `[0]` | Exit statuses that count as a successful hook outcome |
| `abort_on_failure` | boolean | unset | Deprecated compatibility option. If explicitly set to `false`, any exit status is accepted. Prefer `success_codes` for new configs. |

### Example

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]
```

To intentionally check for a command failure, configure the expected non-zero status:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "missing-file.txt"]
success_codes = [1]
```

## Output Example

### Successful Command

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["python", "--version"]
    },
    "success": true
  },
  "status": 0,
  "stdout": "Python 3.11.5\n",
  "stderr": ""
}
```

### Unexpected Status

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["test", "-f", "required-file.txt"]
    },
    "success": false,
    "failure_reason": "command exited with status 1; expected 0"
  },
  "status": 1,
  "stdout": "",
  "stderr": ""
}
```

!!! warning "Failure Behavior"
    Pre-run hook failures are recorded, remaining pre-run hooks still run, and then Capsula stops before running your main command. Post-run hook failures are recorded after the main command; they make `capsula run` fail if the main command succeeded, while preserving the main command's non-zero exit code if it already failed.
