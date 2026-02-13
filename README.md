# Capsula

[![Crate Status](https://img.shields.io/crates/v/capsula.svg)](https://crates.io/crates/capsula)
![Crates.io License](https://img.shields.io/crates/l/capsula)
[![Test Status](https://github.com/ut-issl/capsula/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/ut-issl/capsula/actions)
[![codecov](https://codecov.io/gh/ut-issl/capsula/graph/badge.svg?token=BZXF2PPDM0)](https://codecov.io/gh/ut-issl/capsula)
[![Documentation](https://img.shields.io/badge/docs-capsula-blue)](https://www.space.t.u-tokyo.ac.jp/capsula/)

> [!WARNING]
> This project is in early development. The CLI interface and configuration format may change in future releases.

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
name = "my-project"
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

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
