# Welcome to Capsula

Capsula is a command-line tool that automatically captures and saves information about your command executions. It records what happened, when it happened, and the environment in which it happened.

## What Does Capsula Do?

When you run a command with Capsula, it:

1. **Records the environment** - Captures git state, environment variables, file contents, and system information
2. **Runs your command** - Executes your command normally, capturing its output
3. **Saves everything** - Stores all captured data in an organized directory structure

## Quick Example

Create a `capsula.toml` file:

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

Run your command:

```bash
capsula run python train_model.py
```

Capsula creates an organized directory:

```
.capsula/my-project/2025-01-09/143022-happy-river/
├── _capsula/
│   ├── metadata.json      # What ran, when, and where
│   ├── pre-run.json       # Environment before
│   ├── command.json       # Command output
│   └── post-run.json      # Results after
└── output.txt             # Your output file
```

## Why Use Capsula?

- **Reproducibility** - Capture the exact environment and inputs for every run
- **Traceability** - Know which code version produced which results
- **Auditing** - Generate complete execution records
- **Debugging** - Understand what went wrong by reviewing the complete context

## Getting Started

<div class="grid cards" markdown>

-   :material-download:{ .lg .middle } **Installation**

    ---

    Install Capsula on your system.

    [:octicons-arrow-right-24: Install Capsula](installation.md)

-   :material-rocket-launch:{ .lg .middle } **Getting Started**

    ---

    Run your first command with Capsula.

    [:octicons-arrow-right-24: Quick start tutorial](getting-started.md)

-   :material-cog:{ .lg .middle } **Configuration**

    ---

    Learn how to configure Capsula.

    [:octicons-arrow-right-24: Configuration guide](configuration.md)

-   :material-hook:{ .lg .middle } **Hooks**

    ---

    Explore what Capsula can capture.

    [:octicons-arrow-right-24: Hook reference](hooks.md)

</div>

## Documentation

- **[Installation](installation.md)** - Install and update Capsula
- **[Getting Started](getting-started.md)** - Your first Capsula run
- **[Configuration](configuration.md)** - Configure vaults and hooks
- **[Hooks](hooks.md)** - Available hooks and their options
- **[CLI Reference](cli-reference.md)** - Command-line interface
- **[Server Setup](server-setup.md)** - Optional server component
