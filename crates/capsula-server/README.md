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

### From Source

```bash
# Basic usage with database URL
cargo run -p capsula-server -- --database-url "postgresql://localhost/capsula"

# Or use environment variable
export DATABASE_URL="postgresql://localhost/capsula"
cargo run -p capsula-server

# Custom host and port
cargo run -p capsula-server -- \
  --host 0.0.0.0 \
  --port 8080 \
  --database-url "postgresql://localhost/capsula"

# All options
cargo run -p capsula-server -- \
  --host 0.0.0.0 \
  --port 8080 \
  --database-url "postgresql://localhost/capsula" \
  --storage-path /var/lib/capsula/storage \
  --max-connections 10 \
  --log-level debug
```

The server will start on `http://127.0.0.1:8500` by default.

### Using Docker

```bash
# Start PostgreSQL with Docker
docker run -d \
  --name capsula-postgres \
  -e POSTGRES_DB=capsula \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:16

# Run the server
cargo run -p capsula-server -- \
  --host 0.0.0.0 \
  --port 8500 \
  --database-url "postgresql://postgres:password@localhost:5432/capsula"
```

## Command Line Options

You can view all available options with `--help`:

```bash
cargo run -p capsula-server -- --help
```

```
Web server for managing and viewing Capsula runs

Usage: capsula-server [OPTIONS] --database-url <DATABASE_URL>

Options:
  -H, --host <HOST>
          Host to bind to [env: CAPSULA_HOST=] [default: 127.0.0.1]
  -p, --port <PORT>
          Port to bind to [env: CAPSULA_PORT=] [default: 8500]
  -d, --database-url <DATABASE_URL>
          PostgreSQL connection string [env: DATABASE_URL=]
  -s, --storage-path <STORAGE_PATH>
          Storage directory for captured files [env: STORAGE_PATH=] [default: ./storage]
      --max-connections <MAX_CONNECTIONS>
          Maximum database connections [env: CAPSULA_MAX_CONNECTIONS=] [default: 5]
  -l, --log-level <LOG_LEVEL>
          Log level (error, warn, info, debug, trace) [env: RUST_LOG=] [default: info]
  -h, --help
          Print help
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
| `--database-url` | `-d` | `DATABASE_URL` | (required) | PostgreSQL connection string |
| `--storage-path` | `-s` | `STORAGE_PATH` | `./storage` | Directory for file storage |
| `--max-connections` | | `CAPSULA_MAX_CONNECTIONS` | `5` | Database connection pool size |
| `--log-level` | `-l` | `RUST_LOG` | `info` | Logging level |
| `--max-body-size` | | `CAPSULA_MAX_BODY_SIZE` | `104857600` | Maximum upload body size in bytes (default: 100MB) |

### Configuration Examples

**Using CLI flags only:**
```bash
capsula-server \
  --host 0.0.0.0 \
  --port 8080 \
  --database-url "postgresql://localhost/capsula" \
  --storage-path /data/storage \
  --log-level debug \
  --max-body-size 209715200  # 200MB
```

**Using environment variables only:**
```bash
export DATABASE_URL="postgresql://localhost/capsula"
export CAPSULA_HOST="0.0.0.0"
export CAPSULA_PORT="8080"
export STORAGE_PATH="/data/storage"
export RUST_LOG="debug"
export CAPSULA_MAX_BODY_SIZE="209715200"  # 200MB
capsula-server
```

**Mixed (CLI overrides environment):**
```bash
export DATABASE_URL="postgresql://localhost/capsula"
export CAPSULA_PORT="8080"
# Override port with CLI flag
capsula-server --port 9000  # Will use port 9000, not 8080
```

## API Endpoints

### Runs

- `GET /api/v1/runs` - List all runs
  - Query params: `vault`, `limit`, `offset`
- `POST /api/v1/runs` - Create a new run
- `GET /api/v1/runs/{id}` - Get run details
- `GET /api/v1/runs/{id}/files/{path}` - Download captured file
- `POST /api/v1/upload` - Upload files and hook outputs for a run

### Vaults

- `GET /api/v1/vaults` - List all vaults
- `GET /api/v1/vaults/{name}` - Get vault info

### Health Check

- `GET /health` - Health check endpoint

## Web UI

- `/` - Home page
- `/vaults` - List all vaults
- `/runs` - List all runs
  - Query params: `vault`, `page`
- `/runs/{id}` - View run details

## CLI Integration

Configure the Capsula CLI to push runs to the server:

1. Add server URL to `capsula.toml`:

```toml
[vault]
name = "my-project"

server = "http://localhost:8500"
```

2. Or use environment variable:

```bash
export CAPSULA_SERVER_URL="http://localhost:8500"
```

3. Push a run:

```bash
# By run ID
capsula push 01HQXYZ...

# By run name
capsula push chubby-back
```

4. List vaults on the server:

```bash
capsula vaults list
```

## Database Schema

### Runs Table

Stores metadata about each run:

- `id` (TEXT, PRIMARY KEY): ULID identifier
- `name` (TEXT): Human-readable name
- `timestamp` (TIMESTAMPTZ): When the run occurred
- `command` (TEXT): Command that was executed
- `vault` (TEXT): Vault name
- `project_root` (TEXT): Project root directory
- `exit_code` (INTEGER): Exit code of the command
- `duration_ms` (INTEGER): Duration in milliseconds
- `stdout` (TEXT): Standard output
- `stderr` (TEXT): Standard error
- `created_at` (TIMESTAMPTZ): When the record was created
- `updated_at` (TIMESTAMPTZ): When the record was last updated

### Files Table

Stores captured files:

- `id` (SERIAL, PRIMARY KEY)
- `run_id` (TEXT, FOREIGN KEY): References runs(id)
- `file_path` (TEXT): Relative path in the run directory
- `content` (BYTEA): File content
- `sha256` (TEXT): SHA256 hash of content

### Hooks Table

Stores hook outputs:

- `id` (SERIAL, PRIMARY KEY)
- `run_id` (TEXT, FOREIGN KEY): References runs(id)
- `phase` (TEXT): 'pre' or 'post'
- `hook_output` (JSONB): Hook output data

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
capsula-server --port 8080 --database-url "postgresql://localhost/capsula"
# Or with environment variable
CAPSULA_PORT=8080 capsula-server --database-url "postgresql://localhost/capsula"
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
