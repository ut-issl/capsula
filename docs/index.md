# Welcome to Capsula

Capsula is a command-line tool that automatically captures and saves information about your command executions. Think of it as a "time capsule" for your work - it records what happened, when it happened, and the environment in which it happened.

## What Does Capsula Do?

When you run a command with Capsula, it:

1. **Records the environment** - Captures git state, environment variables, file contents, and system information
2. **Runs your command** - Executes your command normally, capturing its output
3. **Saves everything** - Stores all captured data in an organized directory structure

This makes your work **reproducible** and **auditable** - you can always go back and see exactly what happened during any command execution.

## Why Use Capsula?

### For Researchers and Scientists

- **Reproducibility**: Capture the exact environment and inputs for every experiment
- **Traceability**: Know which code version produced which results
- **Collaboration**: Share complete execution context with colleagues

### For Data Scientists and ML Engineers

- **Experiment tracking**: Automatically record model training parameters and results
- **Debugging**: Understand what went wrong by reviewing the complete execution context
- **Documentation**: Generate audit trails for compliance requirements

### For Software Developers

- **Build auditing**: Track what produced each build artifact
- **Debugging**: Capture system state when issues occur
- **CI/CD integration**: Record deployment contexts

## How It Works

### 1. Create a Configuration File

Create a simple `capsula.toml` file that tells Capsula what to capture:

```toml
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

### 2. Run Your Command

Instead of running:

```bash
python train_model.py
```

Run:

```bash
capsula run python train_model.py
```

### 3. Review the Captured Data

Capsula creates an organized directory with all captured information:

```
.capsula/my-project/2025-01-09/
└── 143022-happy-river/
    ├── _capsula/
    │   ├── metadata.json      # What command ran, when, and where
    │   ├── pre-run.json       # Environment before the command
    │   ├── command.json       # Command output and exit code
    │   └── post-run.json      # Results after the command
    └── output.txt             # Your output file (copied)
```

## Key Features

!!! tip "Easy to Use"
    Simple configuration with plain text TOML files. No programming required.

!!! tip "Non-Invasive"
    Your commands run normally - Capsula just observes and records.

!!! tip "Organized Output"
    Everything saved in a clear directory structure with timestamps.

!!! tip "Flexible"
    Choose what to capture with pre-run and post-run hooks.

!!! tip "Safe"
    Can enforce safety checks (like ensuring clean git state) before running commands.

## Quick Example

Here's a complete example for tracking a Python script:

```toml title="capsula.toml"
[vault]
name = "ml-experiments"

# Before running: check git state and capture config
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Don't run if there are uncommitted changes

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"

# After running: save results
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.png"
mode = "copy"
```

Run your experiment:

```bash
capsula run python train.py --config config.yaml
```

Capsula will:

- Check that your git repository is clean (abort if not)
- Save a copy of `config.yaml`
- Run your Python script
- Copy all PNG files from `results/` to the vault
- Save command output and execution time

## What's Next?

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } **Getting Started**

    ---

    Install Capsula and run your first command in 5 minutes.

    [:octicons-arrow-right-24: Get started](getting-started.md)

-   :material-cog:{ .lg .middle } **Configuration**

    ---

    Learn how to configure Capsula for your needs.

    [:octicons-arrow-right-24: Configuration guide](configuration.md)

-   :material-hook:{ .lg .middle } **Hooks**

    ---

    Explore what Capsula can capture with different hooks.

    [:octicons-arrow-right-24: Hook reference](hooks.md)

-   :material-book-open-variant:{ .lg .middle } **Examples**

    ---

    See real-world examples for different use cases.

    [:octicons-arrow-right-24: View examples](examples.md)

</div>

## Need Help?

- Check the [Troubleshooting guide](troubleshooting.md)
- Read the [FAQ](faq.md)
- Report issues on [GitHub](https://github.com/ut-issl/capsula/issues)
