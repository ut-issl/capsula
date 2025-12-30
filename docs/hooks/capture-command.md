# capture-command

Executes a command and captures its output and exit status.

## Configuration

```toml
# Simple command
[[post-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

# Command with arguments
[[post-run.hooks]]
id = "capture-command"
command = ["ls", "-la", "results/"]

# Abort on failure
[[pre-run.hooks]]
id = "capture-command"
command = ["git", "diff", "--quiet"]
abort_on_failure = true
```

## Parameters

- `command` (required): Array of command and arguments (e.g., `["python", "--version"]`)
- `abort_on_failure` (optional, default: `false`): If `true`, aborts the run if command exits with non-zero status

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

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
  "stdout": "Python 3.11.5\n",
  "stderr": "",
  "status": 0
}
```

### Fields

- `stdout` (string): Standard output from the command
- `stderr` (string): Standard error from the command
- `status` (number): Process exit status code (0 = success)

## Use Cases

### Capture Tool Versions

Document versions of tools used:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "show", "torch"]

[[pre-run.hooks]]
id = "capture-command"
command = ["git", "--version"]
```

### Run Diagnostic Commands

Check system configuration:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi"]

[[pre-run.hooks]]
id = "capture-command"
command = ["df", "-h"]

[[pre-run.hooks]]
id = "capture-command"
command = ["free", "-h"]
```

### Generate Summary Reports

Create summaries after execution:

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["ls", "-lh", "results/"]

[[post-run.hooks]]
id = "capture-command"
command = ["cat", "results/summary.txt"]

[[post-run.hooks]]
id = "capture-command"
command = ["wc", "-l", "results/output.csv"]
```

### Validate Environment

Abort if prerequisites aren't met:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["which", "python3"]
abort_on_failure = true

[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "config.yaml"]
abort_on_failure = true
```

## Examples

### Python Environment Info

```toml
[vault]
name = "python-experiments"

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "-c", "import sys; print(sys.executable)"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "list"]
```

Output:

```json
[
  {
    "__meta": {
      "id": "capture-command",
      "config": {
        "command": ["python", "--version"],
        "abort_on_failure": false
      },
      "success": true
    },
    "stdout": "Python 3.11.5\n",
    "stderr": "",
    "status": 0
  },
  {
    "__meta": {
      "id": "capture-command",
      "config": {
        "command": ["python", "-c", "import sys; print(sys.executable)"],
        "abort_on_failure": false
      },
      "success": true
    },
    "stdout": "/usr/local/bin/python3.11\n",
    "stderr": "",
    "status": 0
  }
]
```

### GPU Information

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi", "--query-gpu=name,memory.total,driver_version", "--format=csv"]
```

Output:

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["nvidia-smi", "--query-gpu=name,memory.total,driver_version", "--format=csv"],
      "abort_on_failure": false
    },
    "success": true
  },
  "stdout": "name, memory.total [MiB], driver_version\nNVIDIA A100-SXM4-40GB, 40960 MiB, 535.129.03\n",
  "stderr": "",
  "status": 0
}
```

## Running Shell Scripts

If you need to run shell scripts with pipes, redirections, or other shell features, invoke the shell explicitly:

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["sh", "-c", "echo 'Files:' && ls -1 results/"]

[[post-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "grep -r 'ERROR' logs/ | wc -l"]
```

## Command Exit Codes

### Successful Command

```json
{
  "stdout": "Success output",
  "stderr": "",
  "status": 0
}
```

### Failed Command

Non-zero exit codes are captured but don't stop execution (unless `abort_on_failure = true`):

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["grep", "NOTFOUND", "file.txt"],
      "abort_on_failure": false
    },
    "success": true
  },
  "stdout": "",
  "stderr": "grep: file.txt: No such file or directory\n",
  "status": 1
}
```

Note: `success: true` in `__meta` indicates the hook executed successfully (captured the command output), not that the command succeeded. The command's exit status is in the `status` field.

## Error Handling

### Command Execution Failure

If the command cannot be executed at all:

```json
{
  "__meta": {
    "id": "capture-command",
    "success": false,
    "error": "Failed to execute command: No such file or directory"
  }
}
```

### Abort on Failure

If `abort_on_failure = true` and the command fails, the run will be aborted after hooks complete:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "required-file.txt"]
abort_on_failure = true
```

If `required-file.txt` doesn't exist, the command exits with status 1, and Capsula will abort before running the main command.

### Non-Fatal Behavior

By default (`abort_on_failure = false`), command failures are non-fatal:

1. Exit code and error output are captured
2. Hook succeeds (marked as `success: true`)
3. Execution continues with remaining hooks

## Performance Considerations

### Long-Running Commands

Commands run synchronously and block execution:

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["sleep", "60"]  # Blocks for 60 seconds
```

Avoid long-running commands in hooks. Instead:

1. Run quick diagnostic commands
2. For analysis, run them separately after `capsula run` completes

### Command Timeouts

Currently, there is no timeout mechanism. Commands run until completion.

## Security Considerations

Commands are executed directly (not via shell unless you explicitly invoke a shell). This provides some protection against command injection, but you should still be careful:

- Avoid constructing commands from untrusted user input
- Be cautious when using environment variables in commands
- When invoking a shell with `-c`, be extra careful about quoting

Safe:

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["echo", "Static text"]  # Safe - no shell interpolation
```

Potentially unsafe:

```toml
[[post-run.hooks]]
id = "capture-command"
# If USER_INPUT contains shell metacharacters, this could be dangerous
command = ["sh", "-c", "echo $USER_INPUT"]
```

## See Also

- [capture_file](capture-file.md) - Capture file contents
- [capture_env](capture-env.md) - Capture environment variables
- [capture_machine](capture-machine.md) - Capture system information
