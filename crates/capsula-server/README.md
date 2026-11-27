# Capsula Server

A web server for storing, browsing, and managing Capsula runs with a PostgreSQL database backend.

## Features

- Web UI for browsing runs and vaults
- REST API for programmatic access
- File upload and storage for captured files
- Hook output storage (pre-run and post-run)
- Vault organization
- Pagination support

## Prerequisites

- PostgreSQL 12 or higher
- Rust 1.91 or higher (for building from source)

## Database Setup

1. Create a PostgreSQL database:

```bash
createdb capsula
```

2. The server will automatically run migrations on startup to create the required tables.

## Running the Server

Three settings are required: the database (`DATABASE_URL`), the object storage (`CAPSULA_STORAGE_URL`, see [Object Storage](#object-storage)) and the authentication mode (`CAPSULA_AUTH_MODE`). Everything else has a default.

### From Source

```bash
# Development: no authentication, files in a local directory
cargo run -p capsula-server -- \
  --database-url "postgresql://localhost/capsula" \
  --storage-url "file:///tmp/capsula-storage" \
  --auth-mode dev

# Or use environment variables
export DATABASE_URL="postgresql://localhost/capsula"
export CAPSULA_STORAGE_URL="file:///tmp/capsula-storage"
export CAPSULA_AUTH_MODE=dev
cargo run -p capsula-server

# Custom host and port
cargo run -p capsula-server -- \
  --host 0.0.0.0 \
  --port 8080 \
  --database-url "postgresql://localhost/capsula" \
  --storage-url "s3://capsula-runs" \
  --auth-mode dev
```

The server will start on `http://127.0.0.1:8500` by default.

`--auth-mode dev` disables authentication: every caller acts as the owner of the `local` organization. Use it only on a loopback address. `--auth-mode oidc` (browser login with Google and/or GitHub, Bearer tokens for the API) is not available yet; the server refuses to start with it until the authentication work of [ut-issl/capsula#1352](https://github.com/ut-issl/capsula/issues/1352) lands.

### Local development with Docker Compose

`compose.yaml` at the repository root defines the services the server depends on:

- `postgres`: PostgreSQL.
- `s3`: an S3-compatible object storage ([versitygw](https://github.com/versity/versitygw)) serving a Docker volume. The bucket named by `S3_BUCKET` is created when the service starts. This service is for development and tests only; use a managed S3-compatible service in production.
- `app`: the server itself, built from `Dockerfile`.

The `just` recipes read their settings from `.env.server`, which is ignored by Git:

```bash
cp .env.server.example .env.server   # adjust ports and credentials if needed
just start-deps                      # PostgreSQL + object storage, waits until healthy
just serve                           # cargo run -p capsula-server with .env.server loaded
just stop-deps                       # stop every compose service
```

`just start-deps` requires [`dotenvx`](https://dotenvx.com/) and Docker. The object storage listens on `S3_PORT` (default `7070`) and accepts the credentials from `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`; `.env.server.example` points `AWS_ENDPOINT_URL` at it and sets `AWS_ALLOW_HTTP=true`, which is only acceptable locally.

Without `just`, the same can be done with Docker Compose directly:

```bash
docker compose --env-file .env.server up postgres s3 --detach --wait
dotenvx run -f .env.server -- cargo run -p capsula-server -- \
  --database-url "postgresql://capsula:capsula_dev@localhost:5432/capsula"
```

## Command Line Options

You can view all available options with `--help` (`-h` for the short form):

```bash
cargo run -p capsula-server -- -h
```

```
Web server for managing and viewing Capsula runs

Usage: capsula-server [OPTIONS] --database-url <DATABASE_URL> --storage-url <STORAGE_URL> --auth-mode <AUTH_MODE>

Options:
  -H, --host <HOST>
          Host to bind to [env: CAPSULA_HOST=] [default: 127.0.0.1]
  -p, --port <PORT>
          Port to bind to [env: CAPSULA_PORT=] [default: 8500]
      --public-url <PUBLIC_URL>
          Public base URL of this server, used for OAuth callbacks and absolute links (for example `https://capsula.example.com`). Required with `--auth-mode oidc` [env: CAPSULA_PUBLIC_URL=]
  -d, --database-url <DATABASE_URL>
          PostgreSQL connection string [env: DATABASE_URL=]
      --max-connections <MAX_CONNECTIONS>
          Maximum database connections [env: CAPSULA_MAX_CONNECTIONS=] [default: 5]
      --storage-url <STORAGE_URL>
          Object storage for captured files: `s3://bucket[/prefix]` (settings from the `AWS_*` environment variables) or `file:///path` (development and tests only) [env: CAPSULA_STORAGE_URL=]
      --auth-mode <AUTH_MODE>
          How callers are authenticated [env: CAPSULA_AUTH_MODE=] [possible values: dev, oidc]
      --oidc-issuer-url <OIDC_ISSUER_URL>
          OIDC issuer URL for browser login (for example `https://accounts.google.com`) [env: CAPSULA_OIDC_ISSUER_URL=]
      --oidc-client-id <OIDC_CLIENT_ID>
          OIDC client id [env: CAPSULA_OIDC_CLIENT_ID=]
      --oidc-client-secret <OIDC_CLIENT_SECRET>
          OIDC client secret [env: CAPSULA_OIDC_CLIENT_SECRET]
      --github-client-id <GITHUB_CLIENT_ID>
          GitHub OAuth App client id [env: CAPSULA_GITHUB_CLIENT_ID=]
      --github-client-secret <GITHUB_CLIENT_SECRET>
          GitHub OAuth App client secret [env: CAPSULA_GITHUB_CLIENT_SECRET]
  -l, --log-level <LOG_LEVEL>
          Log level (error, warn, info, debug, trace) [env: RUST_LOG=] [default: info]
      --max-body-size <MAX_BODY_SIZE>
          Maximum body size for uploads in bytes (default: 100MB) [env: CAPSULA_MAX_BODY_SIZE=] [default: 104857600]
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version
```

## Configuration

The server can be configured via **command-line flags** or **environment variables**. Command-line flags take precedence over environment variables.

**Priority order:** CLI flags > Environment variables > Default values

### Configuration Options

| CLI Flag | Short | Environment Variable | Default | Description |
|----------|-------|---------------------|---------|-------------|
| `--host` | `-H` | `CAPSULA_HOST` | `127.0.0.1` | Host to bind to |
| `--port` | `-p` | `CAPSULA_PORT` | `8500` | Port to bind to |
| `--public-url` | | `CAPSULA_PUBLIC_URL` | (none) | Public base URL of the server (`https://capsula.example.com`), used for OAuth callbacks and absolute links. Required with `--auth-mode oidc` |
| `--database-url` | `-d` | `DATABASE_URL` | (required) | PostgreSQL connection string |
| `--max-connections` | | `CAPSULA_MAX_CONNECTIONS` | `5` | Database connection pool size |
| `--storage-url` | | `CAPSULA_STORAGE_URL` | (required) | Object storage: `s3://bucket[/prefix]` or `file:///path` (development and tests only). See [Object Storage](#object-storage) |
| `--auth-mode` | | `CAPSULA_AUTH_MODE` | (required) | `dev` (no authentication, loopback only) or `oidc` (browser login + Bearer tokens; not available yet) |
| `--oidc-issuer-url` | | `CAPSULA_OIDC_ISSUER_URL` | (none) | OIDC issuer for browser login, for example `https://accounts.google.com`. Set together with the OIDC client id and secret |
| `--oidc-client-id` | | `CAPSULA_OIDC_CLIENT_ID` | (none) | OIDC client id |
| `--oidc-client-secret` | | `CAPSULA_OIDC_CLIENT_SECRET` | (none) | OIDC client secret |
| `--github-client-id` | | `CAPSULA_GITHUB_CLIENT_ID` | (none) | GitHub OAuth App client id. Set together with the secret |
| `--github-client-secret` | | `CAPSULA_GITHUB_CLIENT_SECRET` | (none) | GitHub OAuth App client secret |
| `--log-level` | `-l` | `RUST_LOG` | `info` | Logging level |
| `--max-body-size` | | `CAPSULA_MAX_BODY_SIZE` | `104857600` | Maximum upload body size in bytes (default: 100MB) |

S3 credentials, region and endpoint are not server options: they come from the standard `AWS_*` environment variables (see [Object Storage](#object-storage)).

In `oidc` mode `--public-url` and at least one identity provider (all three `CAPSULA_OIDC_*` variables, or both `CAPSULA_GITHUB_*` variables) are required. A partially configured provider is rejected in every mode. Secrets are never printed; the `Debug` output of the configuration redacts them.

### Configuration Examples

**Using CLI flags only:**
```bash
capsula-server \
  --host 0.0.0.0 \
  --port 8080 \
  --database-url "postgresql://localhost/capsula" \
  --storage-url "s3://capsula-runs" \
  --auth-mode dev \
  --log-level debug \
  --max-body-size 209715200  # 200MB
```

**Using environment variables only:**
```bash
export DATABASE_URL="postgresql://localhost/capsula"
export CAPSULA_STORAGE_URL="s3://capsula-runs"
export CAPSULA_AUTH_MODE="dev"
export CAPSULA_HOST="0.0.0.0"
export CAPSULA_PORT="8080"
export RUST_LOG="debug"
export CAPSULA_MAX_BODY_SIZE="209715200"  # 200MB
capsula-server
```

**Mixed (CLI overrides environment):**
```bash
export DATABASE_URL="postgresql://localhost/capsula"
export CAPSULA_STORAGE_URL="file:///tmp/capsula-storage"
export CAPSULA_AUTH_MODE="dev"
export CAPSULA_PORT="8080"
# Override port with CLI flag
capsula-server --port 9000  # Will use port 9000, not 8080
```

## Object Storage

Captured files are stored in an object store through the [`object_store`](https://docs.rs/object_store) crate. The backend is selected by the `CAPSULA_STORAGE_URL` / `--storage-url` setting:

| URL | Backend | Use |
|-----|---------|-----|
| `s3://bucket` or `s3://bucket/prefix` | Any S3-compatible service (AWS S3, Cloudflare R2, Google Cloud Storage in S3 mode, versitygw, ...) | Production |
| `file:///absolute/path` | A local directory, created if missing | Development and tests only; not supported in production |

S3 settings come from the standard AWS SDK environment variables, read by `object_store`: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_REGION`, `AWS_ENDPOINT_URL`, and `AWS_ALLOW_HTTP` (set to `true` only for local plain-HTTP endpoints). When no static credentials are set, the instance metadata service (IAM roles) and web identity tokens (`AWS_WEB_IDENTITY_TOKEN_FILE`) are used. Profile files under `~/.aws/` are not read.

Captured files are stored content-addressed per organization under the key `{org_id}/sha256/{hash[0..2]}/{hash[2..4]}/{hash}`, where `hash` is the lowercase hex SHA-256 of the content. Identical content uploaded by two organizations is stored twice: deduplication never crosses an organization. A `s3://bucket/prefix` URL prepends `prefix/` to every key.

## API

The single-tenant API v1 (`/api/v1/*`) and the `/vaults` and `/runs` pages have been removed together with their schema. The resource-oriented API v2 (`/api/v2/orgs/{org}/...`) and the `/o/{org}/...` web UI are added resource by resource; the design is tracked in [ut-issl/capsula#1352](https://github.com/ut-issl/capsula/issues/1352). Until then the server serves:

- `GET /health` - Health check (database connectivity)
- `GET /` - Placeholder home page

## Database Schema

The schema is a single baseline migration, `migrations/0001_baseline.sql`. Databases created by earlier versions are not migrated: drop and recreate the database, then refill it with `capsula push --all` from the local `.capsula/` directories.

| Table | Purpose |
|-------|---------|
| `users`, `identities` | Accounts and the Google / GitHub identities linked to them |
| `organizations`, `memberships`, `invitations` | Labs and personal spaces, who belongs to them with which role, invitation links |
| `ssh_keys`, `auth_challenges`, `api_tokens`, `sessions` | Credentials: registered SSH public keys, login nonces, Bearer tokens, browser sessions |
| `vaults` | Projects within an organization (`UNIQUE (org_id, name)`) |
| `runs` | Run metadata; primary key `(org_id, id)` |
| `run_outputs` | Pre/post hook outputs of a run (`UNIQUE (org_id, run_id, phase, hook_index)`) |
| `captured_files` | Captured files of a run; the content lives in object storage under `storage_key` |
| `audit_logs` | Who did what in which organization |

`run_outputs` and `captured_files` reference `runs` through composite foreign keys that include `org_id`, so rows below a run can never point into another organization.

## Development

### Running Tests

```bash
# Run all tests
cargo test -p capsula-server

# Run with output
cargo test -p capsula-server -- --nocapture
```

### Database Migrations

Migrations are embedded in the binary and run automatically on startup. See `crates/capsula-server/migrations/` for migration files.

Compile-time checked SQLx queries need the prepared query data in `.sqlx/`; regenerate it with `just sqlx-prepare` against a database with the migrations applied. CI fails when it is out of date.

### Linting

```bash
# Run all lints
just lint

# Or individually
cargo clippy -p capsula-server
cargo fmt --check
```

## Troubleshooting

### Database Connection Issues

If you see connection errors:

1. Verify PostgreSQL is running:

   ```bash
   pg_isready
   ```

2. Check the DATABASE_URL format:

   ```
   postgresql://[user[:password]@][host][:port][/database]
   ```

3. Verify database exists:

   ```bash
   psql -l | grep capsula
   ```

### Port Already in Use

If port 8500 is already in use, set a different port:

```bash
capsula-server --port 8080
# Or with environment variable
CAPSULA_PORT=8080 capsula-server
```

### Migration Errors

If migrations fail, you can reset the database:

```bash
dropdb capsula
createdb capsula
cargo run -p capsula-server  # Will run migrations
```

## License

MIT OR Apache-2.0
