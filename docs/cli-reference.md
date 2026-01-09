# CLI Reference

Complete reference for all Capsula command-line interface commands and options.

## Global Options

These options work with any Capsula command:

### `--config <PATH>`

Specify a custom configuration file location.

```bash
capsula --config /path/to/custom.toml run python script.py
```

**Default behavior:** Capsula looks for `capsula.toml` in the current directory and parent directories.

### `--help`

Show help information.

```bash
capsula --help
capsula run --help
```

### `--version`

Show the Capsula version.

```bash
capsula --version
```

## Commands

### `run`

Execute a command with full hook capture.

#### Usage

```bash
capsula run [OPTIONS] <COMMAND> [ARGS...]
```

#### Arguments

- `<COMMAND>` - The command to execute
- `[ARGS...]` - Arguments to pass to the command

#### Examples

**Basic usage:**

```bash
capsula run echo "Hello, World!"
```

**With command arguments:**

```bash
capsula run python train.py --epochs 100 --lr 0.01
```

**With custom config:**

```bash
capsula --config experiments.toml run python train.py
```

**With shell commands:**

```bash
capsula run bash -c 'echo "Start"; python script.py; echo "Done"'
```

**With pipes and redirects:**

```bash
capsula run bash -c 'python generate.py | grep "result" > output.txt'
```

#### What Happens

When you run `capsula run <command>`:

1. **Load configuration** - Reads `capsula.toml`
2. **Create run directory** - Creates `.capsula/{vault}/{date}/{time-name}/`
3. **Write metadata** - Saves run ID, name, timestamp, and command
4. **Execute pre-run hooks** - Runs all configured pre-run hooks
5. **Check abort conditions** - Stops if any hook requests abort
6. **Execute command** - Runs your command with captured stdout/stderr
7. **Execute post-run hooks** - Runs all configured post-run hooks
8. **Save results** - Writes all captured data to JSON files

#### Environment Variables

Your command runs with these special environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `CAPSULA_RUN_ID` | Unique run identifier (ULID) | `01K8WSYC91YAE21R7CWHQ4KYN2` |
| `CAPSULA_RUN_NAME` | Human-readable run name | `happy-river` |
| `CAPSULA_RUN_DIRECTORY` | Absolute path to run directory | `/path/.capsula/vault/2025-01-09/143022-happy-river` |
| `CAPSULA_RUN_TIMESTAMP` | ISO 8601 timestamp | `2025-01-09T14:30:22.473+00:00` |
| `CAPSULA_RUN_COMMAND` | Shell-quoted command string | `python train.py --epochs 100` |
| `CAPSULA_PRE_RUN_OUTPUT_PATH` | Path to pre-run.json | `/path/.capsula/.../pre-run.json` |
| `CAPSULA_PROJECT_ROOT` | Project root directory | `/path/to/project` |

**Example using environment variables:**

```bash
capsula run bash -c 'echo "Run: $CAPSULA_RUN_NAME"'
```

```python
# In your Python script
import os

run_id = os.environ['CAPSULA_RUN_ID']
run_name = os.environ['CAPSULA_RUN_NAME']

print(f"Running experiment: {run_name}")
```

#### Exit Codes

- `0` - Command succeeded
- Non-zero - Command failed (returns the command's exit code)

If a hook requests abort (e.g., dirty git repo), Capsula exits before running your command.

---

### `list`

List all captured runs in the vault.

#### Usage

```bash
capsula list [OPTIONS]
```

#### Examples

**List runs:**

```bash
capsula list
```

**With custom config:**

```bash
capsula --config experiments.toml list
```

#### Output Format

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-01-09 14:30:29  happy-river           echo hello
2025-01-09 14:30:28  clever-mountain       python script.py
2025-01-09 14:30:26  quiet-lake            cargo build --release
2025-01-09 14:25:15  swift-breeze          python train.py --epochs 100 --lr 0.01...
```

**Columns:**

- **TIMESTAMP (UTC)** - When the command was executed (in UTC)
- **NAME** - Human-readable generated name for the run
- **COMMAND** - The command that was executed (truncated if too long)

#### Notes

- Runs are sorted by timestamp (most recent first)
- Long commands are truncated with `...`
- Only shows runs from the vault configured in your `capsula.toml`

---

## Configuration File Discovery

Capsula searches for configuration files in this order:

1. **Explicit path with `--config`**
   ```bash
   capsula --config /absolute/path/to/config.toml run echo test
   ```

2. **Current directory**
   ```bash
   ./capsula.toml
   ```

3. **Parent directories** (walks up the tree)
   ```bash
   ../capsula.toml
   ../../capsula.toml
   ...
   ```

**Tip:** Place `capsula.toml` in your project root so it's found from any subdirectory.

## Output Structure

When you run a command, Capsula creates this directory structure:

```
.capsula/{vault-name}/
└── {YYYY-MM-DD}/              # Date directory (UTC)
    └── {HHMMSS-name}/         # Time and run name
        ├── _capsula/          # Capsula metadata
        │   ├── metadata.json  # Run metadata
        │   ├── pre-run.json   # Pre-run hook outputs
        │   ├── command.json   # Command execution results
        │   └── post-run.json  # Post-run hook outputs
        └── [captured files]   # Files copied/moved by hooks
```

### metadata.json

Contains run information:

```json
{
  "id": "01K8WSYC91YAE21R7CWHQ4KYN2",
  "name": "happy-river",
  "command": ["python", "train.py"],
  "timestamp": "2025-01-09T14:30:22.473+00:00",
  "run_dir": "/path/.capsula/vault-name/2025-01-09/143022-happy-river"
}
```

### pre-run.json

Array of pre-run hook outputs:

```json
[
  {
    "__meta": {
      "id": "capture-cwd",
      "config": {},
      "success": true
    },
    "cwd": "/path/to/project"
  },
  {
    "__meta": {
      "id": "capture-git-repo",
      "config": {"path": ".", "allow_dirty": false},
      "success": true
    },
    "working_dir": "/path/to/project",
    "sha": "abc123...",
    "is_dirty": false
  }
]
```

### command.json

Command execution results:

```json
{
  "exit_code": 0,
  "stdout": "Training complete\n",
  "stderr": "",
  "duration": {
    "secs": 42,
    "nanos": 123456789
  }
}
```

### post-run.json

Array of post-run hook outputs (same format as pre-run.json).

## Common Workflows

### Running with Different Configs

Keep multiple config files for different purposes:

```bash
# Run experiment
capsula --config configs/experiment.toml run python train.py

# Run build
capsula --config configs/build.toml run cargo build --release

# Run tests
capsula --config configs/test.toml run pytest
```

### Viewing Recent Runs

```bash
# List runs
capsula list

# View the latest run directory
ls -la .capsula/my-vault/$(date +%Y-%m-%d | head -1)/

# View latest metadata
cat .capsula/my-vault/$(date +%Y-%m-%d)/*/_ capsula/metadata.json | tail -n +2
```

### Debugging Failed Runs

```bash
# Check command output
cat .capsula/my-vault/2025-01-09/143022-happy-river/_capsula/command.json

# Check hook errors
cat .capsula/my-vault/2025-01-09/143022-happy-river/_capsula/pre-run.json | grep "error"
```

### Using Run Name in Scripts

```bash
# Bash
capsula run bash -c 'echo "Results for run: $CAPSULA_RUN_NAME" > results.txt'

# Python
capsula run python -c "
import os
print(f'Run ID: {os.environ[\"CAPSULA_RUN_ID\"]}')
print(f'Run name: {os.environ[\"CAPSULA_RUN_NAME\"]}')
"
```

## Error Messages

### "Configuration file not found"

```
Error: Configuration file not found
```

**Solution:** Create a `capsula.toml` file or use `--config` to specify its location.

### "Failed to parse configuration"

```
Error: Failed to parse configuration file
  --> capsula.toml:5:1
```

**Solution:** Fix the syntax error in your TOML file. Check for:
- Missing closing brackets
- Invalid TOML syntax
- Typos in hook IDs

### "Hook requested abort"

```
Error: Hook 'capture-git-repo' requested abort: Repository has uncommitted changes
```

**Solution:** Either:
- Commit your changes
- Set `allow_dirty = true` in the hook configuration
- Remove the hook if you don't need git checking

### "Command not found"

```
Error: Command not found: python
```

**Solution:** Make sure the command is:
- Installed and in your PATH
- Spelled correctly
- Accessible to your shell

## Best Practices

### 1. Use Descriptive Vault Names

```toml
[vault]
name = "ml-experiments"  # Good
# name = "stuff"         # Bad
```

### 2. Put Configuration in Project Root

Place `capsula.toml` at the root of your project so it's found from any subdirectory.

### 3. Check Exit Codes

```bash
if capsula run python train.py; then
    echo "Training succeeded"
else
    echo "Training failed"
fi
```

### 4. Use Environment Variables

Instead of hard-coding paths, use Capsula's environment variables:

```python
import os
run_dir = os.environ['CAPSULA_RUN_DIRECTORY']
output_path = os.path.join(run_dir, 'results.txt')
```

### 5. Quote Complex Commands

For complex commands with pipes or redirects, use quotes:

```bash
capsula run bash -c 'python generate.py | grep result > output.txt'
```


## Next Steps

- [Configuration Guide](configuration.md) - Learn about all configuration options
- [Hooks Reference](hooks.md) - Explore available hooks
- [CLI Reference](cli-reference.md) - Complete command reference

