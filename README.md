# Capsula

[![Crate Status](https://img.shields.io/crates/v/capsula-cli.svg)](https://crates.io/crates/capsula-cli)
![Crates.io License](https://img.shields.io/crates/l/capsula-cli)
[![Test Status](https://github.com/ut-issl/capsula/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/ut-issl/capsula/actions)
[![codecov](https://codecov.io/gh/ut-issl/capsula/graph/badge.svg?token=BZXF2PPDM0)](https://codecov.io/gh/ut-issl/capsula)

> [!WARNING]
> This project is in early development. The CLI interface and configuration format may change in future releases.

A command-line tool that captures and preserves the context of command executions. Capsula automatically records the state of your project environment before and after running commands, making your workflows reproducible and auditable.

## Quick Start

**Install Capsula:**

```bash
cargo install capsula-cli --locked
```

**Create a configuration file** (`capsula.toml`):

```toml
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-cwd"
```

**Run a command:**

```bash
capsula run python train_model.py
```

**View captured runs:**

```bash
capsula list
```

## Features

- 📸 **Automatic Context Capture** - Records git state, files, environment, and more
- 🔄 **Reproducible Runs** - Complete execution context for debugging and auditing
- 🛡️ **Safety Checks** - Prevent execution on dirty repositories
- 📊 **Structured Output** - JSON-formatted data for easy processing
- 🔧 **Extensible** - Multiple built-in hooks with clean error handling

## Documentation

Complete documentation is available at **[capsula.space.t.u-tokyo.ac.jp](https://www.space.t.u-tokyo.ac.jp/capsula/)**

- [Getting Started](https://www.space.t.u-tokyo.ac.jp/capsula/getting-started/)
- [Configuration Guide](https://www.space.t.u-tokyo.ac.jp/capsula/configuration/)
- [Hook Reference](https://www.space.t.u-tokyo.ac.jp/capsula/hooks/)
- [Examples](https://www.space.t.u-tokyo.ac.jp/capsula/examples/)

## Basic Example

```toml title="capsula.toml"
[vault]
name = "ml-experiments"

# Capture environment before running
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Abort if repo has uncommitted changes

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"

# Capture results after running
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.png"
mode = "copy"
```

Run your experiment:

```bash
capsula run python train.py --config config.yaml
```

Capsula creates an organized directory with all captured data:

```
.capsula/ml-experiments/2025-01-09/143022-happy-river/
├── _capsula/
│   ├── metadata.json      # Run info (ID, command, timestamp)
│   ├── pre-run.json       # Pre-run hook outputs
│   ├── command.json       # Command execution results
│   └── post-run.json      # Post-run hook outputs
├── config.yaml            # Captured input file
└── results/               # Captured output files
    └── plot.png
```

## Available Hooks

- **capture-cwd** - Current working directory
- **capture-env** - Environment variables
- **capture-git-repo** - Git repository state (with dirty check)
- **capture-file** - Files (copy, move, or hash)
- **capture-machine** - System information (CPU, memory, OS)
- **capture-command** - Shell command output
- **notify-slack** - Slack notifications with file attachments

## Use Cases

**For Researchers:**

- Ensure experiment reproducibility
- Track which code version produced which results
- Share complete execution context with colleagues

**For Data Scientists:**

- Automatically record model training parameters
- Debug issues by reviewing execution context
- Generate audit trails for compliance

**For Developers:**

- Track what produced each build artifact
- Capture system state when issues occur
- Document deployment contexts

## Server (Optional)

Capsula includes an optional server component for centralized storage and team collaboration:

```bash
# Start the server
DATABASE_URL="postgresql://localhost/capsula" cargo run -p capsula-server

# Push runs to server
capsula push happy-river
```

See the [Server Setup Guide](https://www.space.t.u-tokyo.ac.jp/capsula/server-setup/) for details.

## Contributing

Contributions are welcome! Please see our [GitHub repository](https://github.com/ut-issl/capsula) for:

- [Issue Tracker](https://github.com/ut-issl/capsula/issues)
- [Discussions](https://github.com/ut-issl/capsula/discussions)
- [Contributing Guidelines](https://github.com/ut-issl/capsula/blob/main/CONTRIBUTING.md)

## License

This project is licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Project Status

> [!NOTE]
> The Python version of Capsula is deprecated and can be found on the `python` branch.

Capsula is actively developed and written in Rust. The current version (0.10.0-alpha) is functional but the API may change before the 1.0 release.
