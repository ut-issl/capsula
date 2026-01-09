# Getting Started

This guide will walk you through using Capsula for the first time. By the end, you'll have run your first command with Capsula and understand how to view the captured data.

!!! info "Before you begin"
    Make sure you have [installed Capsula](installation.md) and can run `capsula --version`.

## Your First Capsula Run

Let's start with a simple example that captures your current directory and runs a basic command.

### Step 1: Create a Configuration File

In any directory, create a file named `capsula.toml`:

```toml title="capsula.toml"
[vault]
name = "my-first-vault"

[[pre-run.hooks]]
id = "capture-cwd"
```

This configuration tells Capsula to:

- Store captured data in a vault named "my-first-vault"
- Before running the command, capture the current working directory

### Step 2: Run a Command

Now run a simple command with Capsula:

```bash
capsula run echo "Hello, Capsula!"
```

You should see output like:

```
Hello, Capsula!
```

The command runs normally, but Capsula has captured information in the background!

### Step 3: View the Captured Data

List all captured runs:

```bash
capsula list
```

You'll see output like:

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-01-09 14:30:22  happy-river           echo "Hello, Capsula!"
```

Each run gets a timestamp (in UTC) and a randomly generated name (like "happy-river") for easy identification.

### Step 4: Explore the Vault

Let's look at what Capsula captured. The vault is stored in `.capsula/`:

```bash
ls .capsula/my-first-vault/
```

You'll see a directory structure like:

```
.capsula/my-first-vault/
└── 2025-01-09/              # Today's date
    └── 143022-happy-river/  # Time and run name
        └── _capsula/        # Capsula metadata
            ├── metadata.json
            ├── pre-run.json
            ├── command.json
            └── post-run.json
```

### Step 5: Inspect the JSON Files

View the run metadata:

```bash
cat .capsula/my-first-vault/2025-01-09/143022-happy-river/_capsula/metadata.json
```

This shows information about the run:

```json
{
  "id": "01K8WSYC91YAE21R7CWHQ4KYN2",
  "name": "happy-river",
  "command": ["echo", "Hello, Capsula!"],
  "timestamp": "2025-01-09T14:30:22.473+00:00"  // UTC
}
```

View the pre-run hook output:

```bash
cat .capsula/my-first-vault/2025-01-09/143022-happy-river/_capsula/pre-run.json
```

This shows the captured current directory:

```json
[
  {
    "__meta": {
      "id": "capture-cwd",
      "config": {},
      "success": true
    },
    "cwd": "/Users/yourname/projects/my-project"
  }
]
```

View the command execution result:

```bash
cat .capsula/my-first-vault/2025-01-09/143022-happy-river/_capsula/command.json
```

This shows the command's output and exit code:

```json
{
  "exit_code": 0,
  "stdout": "Hello, Capsula!\n",
  "stderr": "",
  "duration": {"secs": 0, "nanos": 1986042}
}
```

## A More Practical Example

Now let's try something more useful - capturing git state and a configuration file.

### Step 1: Update Your Configuration

Update your `capsula.toml`:

```toml title="capsula.toml"
[vault]
name = "my-experiments"

# Capture git state before running
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true

# Capture working directory
[[pre-run.hooks]]
id = "capture-cwd"

# Capture an environment variable
[[pre-run.hooks]]
id = "capture-env"
name = "USER"
```

### Step 2: Create a Test File

Create a simple test file:

```bash
echo "test data" > test.txt
```

### Step 3: Run a Command

Run a command that uses this file:

```bash
capsula run cat test.txt
```

### Step 4: Review What Was Captured

Check the new run:

```bash
capsula list
```

Look at the pre-run data:

```bash
# Replace with your actual run directory
cat .capsula/my-experiments/2025-01-09/*/_capsula/pre-run.json
```

You'll see captured data for:

- Git repository state (commit hash, branch, whether it's dirty)
- Current working directory
- The USER environment variable

## Using Post-Run Hooks

Post-run hooks capture data after your command finishes. This is useful for capturing output files.

Update your `capsula.toml`:

```toml title="capsula.toml"
[vault]
name = "my-experiments"

[[pre-run.hooks]]
id = "capture-cwd"

# Capture output file after running
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

Run a command that creates a file:

```bash
capsula run bash -c 'echo "results" > output.txt'
```

Now check the vault - you'll see `output.txt` was copied to the run directory:

```bash
ls .capsula/my-experiments/2025-01-09/*/
```

Output:

```
_capsula/    output.txt
```

## Available Hooks

Capsula provides several hooks to capture different types of information. Hooks can run either before your command (pre-run) or after (post-run).

| Hook | Description | Typical Phase |
|------|-------------|---------------|
| [capture-cwd](hooks/capture-cwd.md) | Captures current working directory | Pre-run |
| [capture-env](hooks/capture-env.md) | Captures environment variables | Pre-run |
| [capture-git-repo](hooks/capture-git-repo.md) | Captures git repository state | Pre-run |
| [capture-file](hooks/capture-file.md) | Captures files (copy/move/hash) | Both |
| [capture-machine](hooks/capture-machine.md) | Captures system information | Pre-run |
| [capture-command](hooks/capture-command.md) | Runs commands and captures output | Both |
| [notify-slack](hooks/notify-slack.md) | Sends Slack notifications | Both |

Click on any hook name to see its detailed documentation and configuration options.

## Working with Environment Variables

When Capsula runs your command, it sets special environment variables you can use:

```bash
capsula run bash -c 'echo "Run ID: $CAPSULA_RUN_ID"; echo "Run name: $CAPSULA_RUN_NAME"'
```

Output:

```
Run ID: 01K8WSYC91YAE21R7CWHQ4KYN2
Run name: happy-river
```

Available variables:

- `CAPSULA_RUN_ID` - Unique identifier for this run
- `CAPSULA_RUN_NAME` - Human-readable name
- `CAPSULA_RUN_DIRECTORY` - Path to the run directory
- `CAPSULA_RUN_TIMESTAMP` - ISO 8601 timestamp (UTC)
- `CAPSULA_RUN_COMMAND` - The command being executed
