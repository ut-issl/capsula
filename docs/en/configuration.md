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

### `server` (optional)

The Capsula server used by `capsula push` and `capsula vaults`. The simplest form is a plain URL:

```toml
server = "https://capsula.example.com"
```

The URL can also be set with the `--server` flag or the `CAPSULA_SERVER_URL` environment variable, which take priority over the config file (in that order). When `[server.headers]` is configured, an override must point at the same origin (scheme, host, and port) as the configured `url` — credentials are never sent to a different origin, and a cross-origin override is rejected with an error.

#### `server.headers` (optional)

When the server sits behind an authenticating reverse proxy (Cloudflare Access, oauth2-proxy, etc.), use the table form to attach HTTP headers to every request:

```toml
[server]
url = "https://capsula.example.com"

[server.headers]
Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }
```

Each entry becomes one HTTP header line. With `CAPSULA_TOKEN=abc123` in the environment, the example above sends `Authorization: Bearer abc123`.

Each header value is one of three sources:

| Form | Meaning |
|------|---------|
| `"literal string"` | Used as-is |
| `{ env = "VAR", prefix = "Bearer " }` | Read from environment variable; optional `prefix` prepended |
| `{ command = "..." }` | Run the command, use trimmed stdout as the value |

Common setups:

```toml
[server.headers]
# Plain bearer token against any authenticating reverse proxy
Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }

# Cloudflare Access, interactive user (JWT from cloudflared login session)
cf-access-token = { command = "cloudflared access token --app=https://capsula.example.com" }

# Cloudflare Access service token (CI)
CF-Access-Client-Id = { env = "CF_ACCESS_CLIENT_ID" }
CF-Access-Client-Secret = { env = "CF_ACCESS_CLIENT_SECRET" }
```

Commands are split with shell-like word splitting and executed directly (no shell) from the project root (the directory containing `capsula.toml`), and their stdout is trimmed of trailing newlines. If a command fails (e.g. an expired login session), the push aborts with the command's stderr.

HTTP redirects are never followed: a redirect (e.g. to a login page) is reported as an error instead of forwarding credentials to the redirect target.

!!! warning "Trust model"
    `command` entries execute arbitrary commands when you run `capsula push` or `capsula vaults`. Only use them with a trusted `capsula.toml` — the same caution that already applies to the `capture-command` hook.

!!! tip "Secrets"
    Never write secret values directly in `capsula.toml`. Reference them with `env` (combined with the top-level `dotenv` option if convenient), or use `command` sources whose credentials live with the external tool (e.g. `~/.cloudflared/`).

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
name = "my-project"
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
name = "research"
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
