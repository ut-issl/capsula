---
icon: material/rocket-launch
---

# Getting Started

This guide will walk you through your first Capsula run and help you understand how it works.

!!! info "Before you begin"
    Make sure you have [installed Capsula](installation.md) and can run `capsula --version`.

## How Capsula Works

When you run a command with Capsula:

1. **Pre-run hooks execute in order** - Capsula runs your configured pre-run hooks sequentially
2. **Your command executes** - Capsula runs your command normally
3. **Post-run hooks execute in order** - Capsula runs your configured post-run hooks sequentially

All outputs are saved in a structured directory called a "vault".

## Your First Run

Create a `capsula.toml` file:

```toml title="capsula.toml"
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true

[[pre-run.hooks]]
id = "capture-cwd"

[[post-run.hooks]]
id = "capture-file"
glob = "*.txt"
mode = "copy"
```

Run a command:

```bash
capsula run echo "Hello, Capsula!" > output.txt
```

List your runs:

```bash
capsula list
```

Output:

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-01-09 14:30:22  happy-river           echo "Hello, Capsula!" > output.txt
```

Each run gets a timestamp (UTC) and a randomly generated name for easy identification.

## Exploring the Vault

Capsula stores everything in `.capsula/`:

```
.capsula/my-project/2025-01-09/143022-happy-river/
├── _capsula/              # Metadata directory
│   ├── metadata.json      # Run info (ID, name, command, timestamp)
│   ├── pre-run.json       # Pre-run hook outputs
│   ├── command.json       # Command execution results
│   └── post-run.json      # Post-run hook outputs
└── output.txt             # Your captured file
```

View what the pre-run hooks recorded:

```bash
cat .capsula/my-project/2025-01-09/143022-happy-river/_capsula/pre-run.json
```

## Available Hooks

| Hook | Description | Typical Phase |
| ------ | ------------- | --------------- |
| [capture-cwd](hooks/capture-cwd.md) | Captures current working directory | Pre-run |
| [capture-env](hooks/capture-env.md) | Captures environment variables | Pre-run |
| [capture-git-repo](hooks/capture-git-repo.md) | Captures git repository state | Pre-run |
| [capture-file](hooks/capture-file.md) | Captures files (copy/move/hash) | Both |
| [capture-machine](hooks/capture-machine.md) | Captures system information | Pre-run |
| [capture-command](hooks/capture-command.md) | Runs commands and captures output | Both |
| [notify-slack](hooks/notify-slack.md) | Sends Slack notifications | Both |

Click on any hook name for detailed configuration options and examples.

## Environment Variables

Capsula sets these environment variables when running your command:

- `CAPSULA_RUN_ID` - Unique identifier
- `CAPSULA_RUN_NAME` - Human-readable name (e.g., "happy-river")
- `CAPSULA_RUN_DIRECTORY` - Path to the run directory
- `CAPSULA_RUN_TIMESTAMP` - ISO 8601 timestamp (UTC)
- `CAPSULA_RUN_COMMAND` - The command being executed

You can use these in your commands:

```bash
capsula run bash -c 'echo "Run: $CAPSULA_RUN_NAME"'
```
