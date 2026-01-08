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
- Rust 1.90 or higher (for building from source)

## Database Setup

1. Create a PostgreSQL database:

```bash
createdb capsula
```

2. The server will automatically run migrations on startup to create the required tables.

## Running the Server

### From Source

```bash
# Set the database URL
export DATABASE_URL="postgresql://localhost/capsula"

# Run the server
cargo run -p capsula-server
```

The server will start on `http://localhost:3000` by default.

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
DATABASE_URL="postgresql://postgres:password@localhost:5432/capsula" \
  cargo run -p capsula-server
```

## Configuration

Environment variables:

- `DATABASE_URL`: PostgreSQL connection string (required)
- `PORT`: Server port (default: 3000)
- `HOST`: Server host (default: 127.0.0.1)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`, `warn`)

Example:

```bash
DATABASE_URL="postgresql://localhost/capsula" \
PORT=8080 \
RUST_LOG=debug \
cargo run -p capsula-server
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

server = "http://localhost:3000"
```

2. Or use environment variable:

```bash
export CAPSULA_SERVER_URL="http://localhost:3000"
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

If port 3000 is already in use, set a different port:

```bash
PORT=8080 cargo run -p capsula-server
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
