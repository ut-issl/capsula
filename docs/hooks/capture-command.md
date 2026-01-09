# capture-command

Runs a shell command and captures its output, exit code, and execution time.

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
| `abort_on_failure` | boolean | `false` | If `true`, Capsula aborts if this command exits with a non-zero code |

### Example

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]
abort_on_failure = false
```

## Output Example

### Successful Command

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["python", "--version"],
      "abort_on_failure": false
    },
    "success": true
  },
  "status": 0,
  "stdout": "Python 3.11.5\n",
  "stderr": "",
  "abort_requested": false
}
```

### Failed Command with Abort

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["test", "-f", "required-file.txt"],
      "abort_on_failure": true
    },
    "success": true
  },
  "status": 1,
  "stdout": "",
  "stderr": "",
  "abort_requested": true
}
```

!!! warning "Abort Behavior"
    When `abort_requested` is `true`, Capsula stops before running your main command.
