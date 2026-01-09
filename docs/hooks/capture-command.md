# capture-command

Runs a shell command and captures its output, exit code, and execution time.

## Use Cases

- **Capture tool versions** - Record Python, Node.js, or other tool versions
- **Run diagnostic commands** - Capture system state with commands like `nvidia-smi` or `df`
- **Execute analysis scripts** - Run quick analysis and save results
- **Validate preconditions** - Check system state before running main command

## Configuration

### Required Options

| Option | Type | Description |
|--------|------|-------------|
| `command` | array | Command and arguments as an array (e.g., `["python", "--version"]`) |

### Optional Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `abort_on_failure` | boolean | `false` | If `true`, Capsula aborts the run if this command exits with a non-zero code |

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

### Failed Command

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["nonexistent-command"],
      "abort_on_failure": false
    },
    "success": true
  },
  "status": 127,
  "stdout": "",
  "stderr": "command not found: nonexistent-command\n",
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

When `abort_requested` is `true`, Capsula stops before running your main command.

### Output Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | number | Exit code (0 = success, non-zero = failure) |
| `stdout` | string | Standard output from the command |
| `stderr` | string | Standard error from the command |
| `abort_requested` | boolean | Whether Capsula should abort (only `true` when `abort_on_failure = true` and command failed) |

## Complete Examples

### Capture Tool Versions

```toml title="capsula.toml"
[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "list"]

[[pre-run.hooks]]
id = "capture-command"
command = ["git", "--version"]
```

### Capture GPU Information

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi"]

[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi", "--query-gpu=name,memory.total,memory.free", "--format=csv"]
```

### Validate Preconditions

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "required-config.json"]
abort_on_failure = true
```

If `required-config.json` doesn't exist, Capsula aborts before running your command.

### Capture System State

```toml
# Disk space
[[pre-run.hooks]]
id = "capture-command"
command = ["df", "-h"]

# Memory usage
[[pre-run.hooks]]
id = "capture-command"
command = ["free", "-h"]

# CPU info
[[pre-run.hooks]]
id = "capture-command"
command = ["lscpu"]
```

## Command Format

Commands are specified as arrays:

```toml
# Correct
command = ["python", "--version"]
command = ["ls", "-la", "/path/to/dir"]
command = ["bash", "-c", "echo hello"]

# Incorrect (strings don't work)
# command = "python --version"  # ❌ Not supported
```

### Running Shell Commands

For shell features (pipes, redirects, etc.), use `bash -c`:

```toml
# With pipes
[[pre-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "ps aux | grep python"]

# With redirects
[[pre-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "cat file.txt 2>&1"]

# With variables
[[pre-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "echo $USER"]
```

## Using Abort on Failure

### Validate File Exists

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "config.yaml"]
abort_on_failure = true
```

### Validate Python Version

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "-c", "import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)"]
abort_on_failure = true
```

### Check Disk Space

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "df -h / | awk 'NR==2 {exit ($5 >= 90)}'"]  # Fail if > 90% full
abort_on_failure = true
```

## Common Patterns

### Pattern: Capture Environment

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "list"]

[[pre-run.hooks]]
id = "capture-command"
command = ["conda", "env", "export"]
```

### Pattern: Validate Preconditions

```toml
# Check required files
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "data.csv"]
abort_on_failure = true

# Check required directories
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-d", "outputs"]
abort_on_failure = true

# Check internet connectivity
[[pre-run.hooks]]
id = "capture-command"
command = ["ping", "-c", "1", "8.8.8.8"]
abort_on_failure = true
```

### Pattern: Capture GPU Info

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi", "-L"]  # List GPUs

[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi", "--query-gpu=gpu_name,memory.total,memory.free,utilization.gpu", "--format=csv"]
```

### Pattern: Analyze Results

```toml
# Count output lines
[[post-run.hooks]]
id = "capture-command"
command = ["wc", "-l", "output.txt"]

# Compute statistics
[[post-run.hooks]]
id = "capture-command"
command = ["python", "analyze.py", "results.csv"]
```

## Tips

### Use Pre-Run for Validation

Put validation commands in pre-run phase:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "config.yaml"]
abort_on_failure = true
```

### Use Post-Run for Analysis

Put analysis commands in post-run phase:

```toml
[[post-run.hooks]]
id = "capture-command"
command = ["python", "summarize.py"]
```

### Capture Both stdout and stderr

Both are captured automatically. Some tools write to stderr:

```toml
# Python --version writes to stderr on some versions
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]
```

### Long-Running Commands

This hook waits for the command to complete. Avoid long-running commands in hooks - run them as your main command instead:

```bash
# Don't do this in a hook
capsula run my-quick-task  # Main task is quick

# Do this instead
capsula run my-long-task  # Long task is the main command
```

## Common Questions

**Q: Can I capture output from my main command?**

Yes! Your main command's output is automatically captured in `command.json`. This hook is for **additional** commands you want to run.

**Q: What if the command doesn't exist?**

The command will fail (non-zero exit code) and stderr will contain an error message. The hook itself still succeeds (the error is recorded).

**Q: Can I run multiple commands in sequence?**

Use `bash -c` with `&&`:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "cd /tmp && ls -la && pwd"]
```

**Q: Can I use environment variables in commands?**

Yes, with `bash -c`:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["bash", "-c", "echo $PATH"]
```

**Q: What's the difference between this and my main command?**

- **Hook commands**: Run before or after your main command to capture environment/results
- **Main command**: The primary task you're running (specified in `capsula run <command>`)

**Q: Can I access Capsula environment variables?**

Hook commands run in a different context and don't have access to Capsula environment variables (those are only set for your main command).

**Q: What happens if abort_on_failure is true but the hook itself fails?**

If the command exits with a non-zero code and `abort_on_failure = true`, Capsula aborts the run. The hook output shows `abort_requested: true`.

**Q: Are there security concerns?**

Yes - be careful with command injection if constructing commands from user input. Use static commands in your config file.

**Q: Can I pass arguments from Capsula to the command?**

Not directly in hook commands. If you need dynamic behavior, put logic in a script and call it:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["./scripts/check-environment.sh"]
```

## Related Hooks

- [capture-env](capture-env.md) - Capture environment variables
- [capture-machine](capture-machine.md) - Capture system information
- [capture-git-repo](capture-git-repo.md) - Capture git state (with optional abort)

[:octicons-arrow-left-24: Back to Configuration](../configuration.md#available-hook-types)
