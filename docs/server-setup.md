# Server Setup

Capsula includes an optional server component that allows you to store and browse your runs in a centralized location. This is useful for teams who want to share runs or access them from multiple machines.

!!! info "Optional Feature"
    The server is completely optional. You can use Capsula without it - runs are stored locally in `.capsula/` directories by default.

## What Does the Server Do?

The Capsula server provides:

- **Web UI** - Browse runs and vaults in your browser
- **Centralized storage** - Store runs from multiple machines in one place
- **Team sharing** - Share runs with teammates
- **REST API** - Programmatically access run data

## Prerequisites

Before setting up the server, you need:

- **PostgreSQL** (version 12 or higher)
- **Rust** (version 1.90 or higher) - if building from source

## Quick Start

### Step 1: Install PostgreSQL

#### On macOS (with Homebrew)

```bash
brew install postgresql@16
brew services start postgresql@16
```

#### On Ubuntu/Debian

```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

#### On Windows

Download and install from [postgresql.org](https://www.postgresql.org/download/windows/)

#### Using Docker

```bash
docker run -d \
  --name capsula-postgres \
  -e POSTGRES_DB=capsula \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 \
  postgres:16
```

### Step 2: Create a Database

```bash
createdb capsula
```

Or if using a password:

```bash
createdb -h localhost -U postgres capsula
```

### Step 3: Run the Server

```bash
# Set the database connection
export DATABASE_URL="postgresql://localhost/capsula"

# Run the server
cargo run -p capsula-server
```

The server will start on `http://localhost:3000`.

!!! success "Server is Running"
    Open http://localhost:3000 in your browser to see the web interface!

## Configuration

The server is configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | (required) |
| `PORT` | Server port | `3000` |
| `HOST` | Server host address | `127.0.0.1` |
| `RUST_LOG` | Logging level | `info` |

### Example: Custom Port

```bash
DATABASE_URL="postgresql://localhost/capsula" \
PORT=8080 \
cargo run -p capsula-server
```

### Example: Debug Logging

```bash
DATABASE_URL="postgresql://localhost/capsula" \
RUST_LOG=debug \
cargo run -p capsula-server
```

## Database Connection String Format

The `DATABASE_URL` follows this format:

```
postgresql://[user[:password]@][host][:port][/database]
```

### Examples

**Local database (no password):**
```
postgresql://localhost/capsula
```

**Local database with user and password:**
```
postgresql://user:password@localhost/capsula
```

**Remote database:**
```
postgresql://user:password@db.example.com:5432/capsula
```

**Using environment-specific users:**
```
postgresql://postgres:secret@localhost:5432/capsula
```

## Using the Server with Capsula CLI

Once the server is running, you can push runs to it from the CLI.

### Configure Server URL

Add the server URL to your `capsula.toml`:

```toml
[vault]
name = "my-project"

[server]
url = "http://localhost:3000"
```

Or set an environment variable:

```bash
export CAPSULA_SERVER_URL="http://localhost:3000"
```

### Push a Run

After running a command, push it to the server:

```bash
# Run a command
capsula run python train.py

# Push by run name (from capsula list)
capsula push happy-river

# Or push by run ID
capsula push 01HQXYZ...
```

### List Server Vaults

```bash
capsula vaults list
```

## Web Interface

The server provides a web interface for browsing runs:

- **Home**: `http://localhost:3000/`
- **List vaults**: `http://localhost:3000/vaults`
- **List runs**: `http://localhost:3000/runs`
- **View run**: `http://localhost:3000/runs/{run-id}`
- **Filter by vault**: `http://localhost:3000/runs?vault=my-project`

## API Endpoints

If you want to integrate with the server programmatically, it provides a REST API:

### Runs API

```bash
# List all runs
curl http://localhost:3000/api/v1/runs

# Filter by vault
curl http://localhost:3000/api/v1/runs?vault=my-project

# Pagination
curl http://localhost:3000/api/v1/runs?limit=20&offset=40

# Get run details
curl http://localhost:3000/api/v1/runs/{run-id}

# Download a captured file
curl http://localhost:3000/api/v1/runs/{run-id}/files/results/output.txt
```

### Vaults API

```bash
# List all vaults
curl http://localhost:3000/api/v1/vaults

# Get vault info
curl http://localhost:3000/api/v1/vaults/my-project
```

### Health Check

```bash
curl http://localhost:3000/health
```

## Database Schema

The server uses three main tables:

### Runs Table

Stores metadata about each run:

- Run ID, name, timestamp
- Command and exit code
- Vault name
- stdout and stderr
- Execution duration

### Files Table

Stores captured files:

- File path and content
- SHA256 hash
- Associated run ID

### Hooks Table

Stores hook outputs:

- Pre-run and post-run hook data
- Associated run ID
- JSON hook output

## Production Deployment

For production use, consider:

### 1. Use a Process Manager

Keep the server running with systemd, supervisor, or similar:

**Example systemd service** (`/etc/systemd/system/capsula-server.service`):

```ini
[Unit]
Description=Capsula Server
After=postgresql.service

[Service]
Type=simple
User=capsula
WorkingDirectory=/opt/capsula
Environment="DATABASE_URL=postgresql://capsula:password@localhost/capsula"
Environment="PORT=3000"
Environment="RUST_LOG=info"
ExecStart=/opt/capsula/capsula-server
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable capsula-server
sudo systemctl start capsula-server
```

### 2. Use a Reverse Proxy

Put the server behind nginx or Apache:

**Example nginx configuration:**

```nginx
server {
    listen 80;
    server_name capsula.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 3. Secure the Database

- Use a strong database password
- Restrict database access to localhost or specific IPs
- Enable SSL for database connections

### 4. Enable HTTPS

Use Let's Encrypt or similar for HTTPS:

```bash
sudo certbot --nginx -d capsula.example.com
```

### 5. Backup the Database

Regularly backup your PostgreSQL database:

```bash
# Backup
pg_dump capsula > capsula_backup.sql

# Restore
psql capsula < capsula_backup.sql
```

## Troubleshooting

### Server Won't Start

**Error**: `connection refused`

Check if PostgreSQL is running:

```bash
# Check status
pg_isready

# Start if not running (macOS)
brew services start postgresql@16

# Start if not running (Linux)
sudo systemctl start postgresql
```

---

**Error**: `database "capsula" does not exist`

Create the database:

```bash
createdb capsula
```

---

**Error**: `authentication failed`

Check your `DATABASE_URL` includes the correct username and password.

### Port Already in Use

**Error**: `Address already in use`

Either:

1. Stop the other service using port 3000
2. Use a different port:

```bash
PORT=8080 cargo run -p capsula-server
```

### Can't Connect from Other Machines

If running on a server but can't connect from other machines:

1. **Change host to 0.0.0.0:**

```bash
HOST=0.0.0.0 cargo run -p capsula-server
```

2. **Check firewall rules:**

```bash
# Allow port 3000 (Ubuntu/Debian)
sudo ufw allow 3000

# Allow port 3000 (CentOS/RHEL)
sudo firewall-cmd --add-port=3000/tcp --permanent
sudo firewall-cmd --reload
```

### Migration Errors

If database migrations fail, reset the database:

```bash
# Drop and recreate
dropdb capsula
createdb capsula

# Restart server (will run migrations)
cargo run -p capsula-server
```

### Viewing Logs

Enable debug logging to see what's happening:

```bash
RUST_LOG=debug cargo run -p capsula-server
```

## Uninstalling

To remove the server and database:

```bash
# Stop the server (Ctrl+C if running in terminal)

# Drop the database
dropdb capsula

# Uninstall PostgreSQL (if desired)
# macOS:
brew services stop postgresql@16
brew uninstall postgresql@16

# Ubuntu/Debian:
sudo apt remove postgresql postgresql-contrib
```


## Next Steps

- [Configuration Guide](configuration.md) - Learn about all configuration options
- [Hooks Reference](hooks.md) - Explore available hooks
- [CLI Reference](cli-reference.md) - Complete command reference
