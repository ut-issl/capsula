---
icon: material/console
---

# CLI Reference

Complete reference for Capsula command-line interface.

## Global Options

### `--config <PATH>`

Specify a custom configuration file.

```bash
capsula --config /path/to/custom.toml run python script.py
```

Default: Looks for `capsula.toml` in current directory and parent directories.

### `--vault-path <PATH>`

Override the vault path. Can also be set via the `CAPSULA_VAULT_PATH` environment variable (after dotenv loading).

```bash
capsula --vault-path /custom/vault/path list
```

### `--version`

Show Capsula version.

```bash
capsula --version
```

### `--help`

Show help information.

```bash
capsula --help
capsula run --help
```

## `capsula run`

Execute a command with hook capture.

### Usage

```bash
capsula run <COMMAND> [ARGS...]
```

### Examples

```bash
capsula run echo "Hello, World!"
capsula run python train.py --epochs 100
capsula run bash -c 'python generate.py | grep result > output.txt'
```

### Environment Variables

Your command runs with these environment variables:

| Variable | Description | Example |
| ---------- | ------------- | --------- |
| `CAPSULA_RUN_ID` | Unique run identifier (ULID) | `01K8WSYC91YAE21R7CWHQ4KYN2` |
| `CAPSULA_RUN_NAME` | Human-readable run name | `happy-river` |
| `CAPSULA_RUN_DIRECTORY` | Absolute path to run directory | `/path/.capsula/vault/2025-01-09/143022-happy-river` |
| `CAPSULA_RUN_TIMESTAMP` | ISO 8601 timestamp (UTC) | `2025-01-09T14:30:22.473+00:00` |
| `CAPSULA_RUN_COMMAND` | Shell-quoted command string | `python train.py --epochs 100` |
| `CAPSULA_PRE_RUN_OUTPUT_PATH` | Path to pre-run.json | `/path/.capsula/.../pre-run.json` |
| `CAPSULA_PROJECT_ROOT` | Project root directory | `/path/to/project` |

Example using environment variables:

```bash
capsula run bash -c 'echo "Run: $CAPSULA_RUN_NAME"'
```

## `capsula run-start`

Start a manual run: create the run directory and execute pre-run hooks.

Use this when you want to capture context before and after an external process
(e.g., a GUI-triggered analysis) that capsula does not manage directly.
The auto-generated run name is printed to stdout so callers can capture it,
then later finalize the run with `run-end`.

### Usage

```bash
capsula run-start
```

### Example

```bash
# Start a manual run and capture the name
name=$(capsula run-start)

# ... perform your external process ...

# Finalize the run
capsula run-end "$name"
```

## `capsula run-end`

End a manual run: execute post-run hooks for an existing run.

Finalizes a run previously started with `run-start`. This command will fail
if the run has already been finalized (i.e., `post-run.json` already exists)
or if any post-run hook fails after `post-run.json` is written.

### Usage

```bash
capsula run-end <RUN_NAME>
```

### Example

```bash
capsula run-end happy-river
```

## `capsula list`

List all captured runs.

### Usage

```bash
capsula list
```

### Output Format

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-01-09 14:30:29  happy-river           echo hello
2025-01-09 14:30:28  clever-mountain       python script.py
2025-01-09 14:30:26  quiet-lake            cargo build --release
```

## `capsula show`

Show detailed information about a specific run, including metadata, command
result, and hook summaries.

### Usage

```bash
capsula show <RUN_NAME>
capsula show <RUN_NAME> --json
```

### Options

| Option | Description |
| -------- | ------------- |
| `--json` | Output the full run data as JSON |

### Example

```bash
$ capsula show happy-river
Run:       happy-river
ID:        01K8WSYC91YAE21R7CWHQ4KYN2
Timestamp: 2025-01-09 14:30:22 UTC
Command:   python train.py --epochs 100
Directory: /path/.capsula/vault/2025-01-09/143022-happy-river
Result:    exit 0 (1.23s)

Pre-run hooks:
  [ok]     capture-git-repo
  [ok]     capture-env
Post-run hooks:
  [ok]     capture-file
```

## `capsula run-dir`

Print the run directory for a run name.

### Usage

```bash
capsula run-dir <RUN_NAME>
```

### Example

```bash
capsula run-dir happy-river
```

## `capsula push`

Push run data to a capsula server.

### Usage

```bash
capsula push <RUN_ID_OR_NAME>
capsula push --all
```

### Options

| Option | Description |
| -------- | ------------- |
| `--all` | Push all runs in the vault |
| `--server <URL>` | Server URL (can also be set via `CAPSULA_SERVER_URL` env var or `server` field in `capsula.toml`) |

### Examples

```bash
# Push a single run by name
capsula push happy-river

# Push a single run by ID
capsula push 01K8WSYC91YAE21R7CWHQ4KYN2

# Push all runs
capsula push --all

# Push to a specific server
capsula push happy-river --server https://capsula.example.com
```

## `capsula tui`

Launch an interactive terminal UI for starting and ending runs.

The TUI provides a visual interface for managing manual runs, as an alternative
to using `run-start` and `run-end` from the command line.

### Usage

```bash
capsula tui
```

## `capsula vaults`

Manage vaults on the server.

### `capsula vaults list`

List all vaults on the server.

```bash
capsula vaults list
capsula vaults list --server https://capsula.example.com
```

#### Options

| Option | Description |
| -------- | ------------- |
| `--server <URL>` | Server URL (can also be set via `CAPSULA_SERVER_URL` env var or `server` field in `capsula.toml`) |
