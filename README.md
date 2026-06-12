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
│   ├── metadata.json           # What ran, when, and where
│   ├── pre-run.json            # Environment before
│   ├── command.json            # Command output
│   └── post-run.json           # Results after
├── pre-0-capture-git-repo/     # Per-hook artifact directory
└── post-0-capture-file/        # Per-hook artifact directory
    └── output.txt              # Your output file
```

Hooks that produce file artifacts (e.g., `capture-file`, `capture-git-repo`) each
get a dedicated subdirectory named `{phase}-{index}-{hook_id}/`, which prevents
filename collisions between hooks.

## Why Use Capsula?

- **Reproducibility** - Capture the exact environment and inputs for every run
- **Traceability** - Know which code version produced which results
- **Auditing** - Generate complete execution records
- **Debugging** - Understand what went wrong by reviewing the complete context

## Server and Web UI

Capsula also includes `capsula-server`, a PostgreSQL-backed web server for storing,
browsing, and sharing run records beyond the local `.capsula` directory. It provides:

- A web UI for browsing vaults, runs, hook outputs, and captured files
- A REST API for programmatic access to runs, vaults, and file downloads
- CLI integration via `capsula push` and `capsula vaults list`

Run data can be pushed to a server configured with `--server`, `CAPSULA_SERVER_URL`,
or the top-level `server = "https://capsula.example.com"` field in `capsula.toml`.
See `crates/capsula-server/README.md` for setup details.

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
