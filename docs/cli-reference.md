# CLI Reference

Complete reference for Capsula command-line interface.

## Global Options

### `--config <PATH>`

Specify a custom configuration file.

```bash
capsula --config /path/to/custom.toml run python script.py
```

Default: Looks for `capsula.toml` in current directory and parent directories.

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
