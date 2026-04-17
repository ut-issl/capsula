use capsula_server::{build_app, create_pool};
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// Web server for managing and viewing Capsula runs
#[derive(Parser, Debug)]
#[command(name = "capsula-server", version, about)]
struct Args {
    /// Host to bind to
    #[arg(short = 'H', long, env = "CAPSULA_HOST", default_value = "127.0.0.1")]
    host: String,

    /// Port to bind to
    #[arg(short, long, env = "CAPSULA_PORT", default_value = "8500")]
    port: u16,

    #[expect(clippy::doc_markdown, reason = "It is not a variable but a term")]
    /// PostgreSQL connection string
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// Storage directory for captured files
    #[arg(short, long, env = "STORAGE_PATH", default_value = "./storage")]
    storage_path: String,

    /// Maximum database connections
    #[arg(long, env = "CAPSULA_MAX_CONNECTIONS", default_value = "5")]
    max_connections: u32,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short, long, env = "RUST_LOG", default_value = "info")]
    log_level: String,

    /// Maximum body size for uploads in bytes (default: 100MB)
    #[arg(long, env = "CAPSULA_MAX_BODY_SIZE", default_value = "104857600")]
    max_body_size: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logging with configured level
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::new(&args.log_level))
        .init();

    info!("Starting Capsula Server");
    info!("Host: {}", args.host);
    info!("Port: {}", args.port);
    info!("Storage path: {}", args.storage_path);
    info!("Max database connections: {}", args.max_connections);
    info!("Max body size: {} bytes", args.max_body_size);
    info!("Log level: {}", args.log_level);

    info!("Connecting to database...");

    let pool = match create_pool(&args.database_url, args.max_connections).await {
        Ok(pool) => {
            info!("Database connection established");
            pool
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            error!(
                "\nPlease ensure:\n\
                 1. PostgreSQL is running\n\
                 2. The database exists (try: createdb capsula)\n\
                 3. The connection string is correct\n\
                 4. You have proper permissions"
            );
            std::process::exit(1);
        }
    };

    // Run database migrations
    info!("Running database migrations...");
    let migrations = sqlx::migrate!("./migrations");
    if let Err(e) = migrations.run(&pool).await {
        error!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }
    info!("Database migrations completed");

    let app = build_app(pool, args.storage_path.into(), args.max_body_size);

    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to {}: {}", bind_addr, e);
            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!(
                    "Port {} is already in use. Try a different port with --port <PORT>",
                    args.port
                );
            } else if e.kind() == std::io::ErrorKind::AddrNotAvailable {
                error!(
                    "Cannot bind to host {}. Try --host 0.0.0.0 or --host 127.0.0.1",
                    args.host
                );
            } else {
                error!("Check your network configuration and try again");
            }
            std::process::exit(1);
        }
    };

    info!("Server listening on http://{}", bind_addr);

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
