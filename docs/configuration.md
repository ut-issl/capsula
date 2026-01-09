---
icon: material/cog
---

# Configuration

Capsula is configured using a `capsula.toml` file. This page documents all configuration options.

## Configuration File Location

Capsula looks for `capsula.toml` in the following order:

1. Path specified with `--config` flag
2. Current directory (`./capsula.toml`)
3. Parent directories (walking up the tree)

!!! tip
    Place `capsula.toml` in your project root so it works from any subdirectory.

## Basic Structure

```toml
[vault]
name = "vault-name"

[[pre-run.hooks]]
id = "hook-type"
# hook configuration...

[[post-run.hooks]]
id = "hook-type"
# hook configuration...
```

## Top-Level Options

### `dotenv` (optional)

Load environment variables from a `.env` file.

```toml
dotenv = ".env"
```

Relative paths are resolved from the directory containing `capsula.toml`.

## Vault Configuration

The `[vault]` section defines where Capsula stores captured data.

### `name` (required)

The vault name. Creates a subdirectory under `.capsula/`.

```toml
[vault]
name = "ml-experiments"
```

Creates: `.capsula/ml-experiments/`

### `path` (optional)

Custom path for the vault. Can be absolute or relative.

```toml
[vault]
name = "experiments"
path = "/absolute/path/to/vault"
```

## Hook Configuration

Hooks are configured in two sections: `pre-run` and `post-run`.
For the configuration of each hook, refer to its specific documentation.

### Pre-Run Hooks

Pre-run hooks are executed **before** your command, in the order listed.

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-cwd"
```

### Post-Run Hooks

Post-run hooks are executed **after** your command, in the order listed.

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

## Complete Example

```toml title="capsula.toml"
dotenv = ".env"

[vault]
name = "research-experiments"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"

[[post-run.hooks]]
id = "capture-file"
glob = "results/*.json"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

## Multiple Configurations

You can use different config files for different purposes:

```bash
capsula --config experiments.toml run python train.py
capsula --config builds.toml run cargo build
```
