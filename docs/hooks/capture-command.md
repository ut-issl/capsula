# capture_command

Executes a shell command and captures its output, exit code, and duration.

## Configuration

```toml
# Simple command
[[post_run]]
type = "capture_command"
command = "python --version"

# Custom shell
[[post_run]]
type = "capture_command"
command = "ls -la results/"
shell = "/bin/bash"

# Multi-line command
[[post_run]]
type = "capture_command"
command = """
echo "Experiment Summary:"
cat results/metrics.json | jq '.accuracy'
"""
```

## Parameters

- `command` (required): Shell command to execute
- `shell` (optional, default: `"/bin/sh"`): Shell interpreter to use

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture_command",
    "config": {
      "command": "python --version",
      "shell": "/bin/sh"
    },
    "success": true
  },
  "command": "python --version",
  "exit_code": 0,
  "stdout": "Python 3.11.5\n",
  "stderr": "",
  "duration_ms": 42
}
```

### Fields

- `command` (string): Command that was executed
- `exit_code` (number): Process exit code (0 = success)
- `stdout` (string): Standard output from the command
- `stderr` (string): Standard error from the command
- `duration_ms` (number): Execution time in milliseconds

## Use Cases

### Capture Tool Versions

Document versions of tools used:

```toml
[[pre_run]]
type = "capture_command"
command = "python --version"

[[pre_run]]
type = "capture_command"
command = "pip list | grep torch"

[[pre_run]]
type = "capture_command"
command = "git --version"
```

### Run Diagnostic Commands

Check system configuration:

```toml
[[pre_run]]
type = "capture_command"
command = "nvidia-smi"

[[pre_run]]
type = "capture_command"
command = "df -h"

[[pre_run]]
type = "capture_command"
command = "free -h"
```

### Generate Summary Reports

Create summaries after execution:

```toml
[[post_run]]
type = "capture_command"
command = "ls -lh results/"

[[post_run]]
type = "capture_command"
command = "cat results/summary.txt"

[[post_run]]
type = "capture_command"
command = "wc -l results/*.csv"
```

### Process Results

Extract specific metrics:

```toml
[[post_run]]
type = "capture_command"
command = "jq '.accuracy' results/metrics.json"

[[post_run]]
type = "capture_command"
command = "tail -n 10 training.log"
```

## Examples

### Python Environment Info

```toml
[vault]
name = "python-experiments"

[[pre_run]]
type = "capture_command"
command = "python --version"

[[pre_run]]
type = "capture_command"
command = "python -c 'import sys; print(sys.executable)'"

[[pre_run]]
type = "capture_command"
command = "pip list"
```

Output:

```json
[
  {
    "__meta": { "id": "capture_command", "success": true },
    "command": "python --version",
    "exit_code": 0,
    "stdout": "Python 3.11.5\n",
    "stderr": "",
    "duration_ms": 38
  },
  {
    "__meta": { "id": "capture_command", "success": true },
    "command": "python -c 'import sys; print(sys.executable)'",
    "exit_code": 0,
    "stdout": "/usr/local/bin/python3.11\n",
    "stderr": "",
    "duration_ms": 42
  }
]
```

### GPU Information

```toml
[[pre_run]]
type = "capture_command"
command = "nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv"
```

Output:

```json
{
  "__meta": { "id": "capture_command", "success": true },
  "command": "nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv",
  "exit_code": 0,
  "stdout": "name, memory.total [MiB], driver_version\nNVIDIA A100-SXM4-40GB, 40960 MiB, 535.129.03\n",
  "stderr": "",
  "duration_ms": 156
}
```

### Results Summary

```toml
[[post_run]]
type = "capture_command"
command = """
echo "=== Results Summary ==="
echo "Files created:"
ls -1 results/
echo ""
echo "Total size:"
du -sh results/
"""
```

### Multi-Step Analysis

```toml
[[post_run]]
type = "capture_command"
command = """
echo "Best accuracy:"
cat results/metrics.json | jq -r '.best_accuracy'
echo "Training time:"
cat results/metrics.json | jq -r '.total_time_seconds'
"""
shell = "/bin/bash"
```

## Shell Selection

### Default Shell (`/bin/sh`)

```toml
[[post_run]]
type = "capture_command"
command = "echo $SHELL"
```

### Bash-Specific Features

```toml
[[post_run]]
type = "capture_command"
command = "echo ${BASH_VERSION}"
shell = "/bin/bash"
```

### Custom Shell

```toml
[[post_run]]
type = "capture_command"
command = "echo 'Hello from zsh'"
shell = "/bin/zsh"
```

## Multi-Line Commands

Use TOML triple-quoted strings:

```toml
[[post_run]]
type = "capture_command"
command = """
#!/bin/bash
set -e

echo "Analyzing results..."

if [ -f "results/metrics.json" ]; then
    jq '.' results/metrics.json
else
    echo "No metrics file found"
fi
"""
shell = "/bin/bash"
```

## Command Exit Codes

### Successful Command

```json
{
  "exit_code": 0,
  "stdout": "Success output",
  "stderr": ""
}
```

### Failed Command

Non-zero exit codes are captured but don't stop execution:

```json
{
  "__meta": {
    "id": "capture_command",
    "success": true,
    "config": { "command": "grep NOTFOUND file.txt" }
  },
  "command": "grep NOTFOUND file.txt",
  "exit_code": 1,
  "stdout": "",
  "stderr": "grep: file.txt: No such file or directory\n",
  "duration_ms": 5
}
```

Note: `success: true` indicates the hook executed successfully (captured the command output), not that the command succeeded.

## Error Handling

### Command Execution Failure

If the command cannot be executed at all:

```json
{
  "__meta": {
    "id": "capture_command",
    "success": false,
    "error": "Failed to execute command: No such file or directory"
  }
}
```

### Shell Not Found

```json
{
  "__meta": {
    "id": "capture_command",
    "success": false,
    "error": "Shell not found: /bin/nonexistent"
  }
}
```

### Non-Fatal Behavior

Command failures are non-fatal:

1. Exit code and error output are captured
2. Hook succeeds (marked as `success: true`)
3. Execution continues with remaining hooks

## Performance Considerations

### Long-Running Commands

Commands run synchronously and block execution:

```toml
[[post_run]]
type = "capture_command"
command = "sleep 60"  # Blocks for 60 seconds
```

Avoid long-running commands in hooks. Instead:

1. Run quick diagnostic commands
2. For analysis, run them separately after `capsula run` completes

### Command Timeouts

Currently, there is no timeout mechanism. Commands run until completion.

## Security Considerations

!!! warning "Command Injection"
    Commands are executed via shell. Be careful with:

    - User input in commands
    - Environment variables in commands
    - Dynamic command construction

Avoid:

```toml
# Unsafe if $USER_INPUT is untrusted
command = "echo $USER_INPUT"
```

Prefer:

```toml
# Safe - no variable interpolation
command = "echo 'Static text'"
```

## See Also

- [capture_file](capture-file.md) - Capture file contents
- [capture_env](capture-env.md) - Capture environment variables
- [capture_machine](capture-machine.md) - Capture system information
