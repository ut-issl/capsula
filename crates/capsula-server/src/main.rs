use capsula_server::config::{AuthMode, Config};
use capsula_server::storage::Storage;
use capsula_server::{AppState, build_app, create_pool, migrator};
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let config = Config::parse();

    // Initialize logging with configured level
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::new(&config.log_level))
        .init();

    if let Err(e) = config.validate() {
        error!("Invalid configuration: {e}");
        std::process::exit(2);
    }

    info!("Starting Capsula Server");
    info!("Host: {}", config.host);
    info!("Port: {}", config.port);
    if let Some(public_url) = &config.public_url {
        info!("Public URL: {public_url}");
    }
    info!("Storage URL: {}", config.storage_url);
    info!("Auth mode: {}", config.auth_mode);
    info!("Max database connections: {}", config.max_connections);
    info!("Max body size: {} bytes", config.max_body_size);
    info!("Log level: {}", config.log_level);

    if config.auth_mode == AuthMode::Oidc {
        // Authentication is added by the SaaS MVP work (ut-issl/capsula#1356).
        // Refuse to start rather than serve an unauthenticated server that
        // the operator believes to be protected.
        error!("--auth-mode oidc is not implemented yet; only `dev` is available for now");
        std::process::exit(2);
    }

    let storage = match Storage::from_url(&config.storage_url) {
        Ok(storage) => storage,
        Err(e) => {
            error!("Failed to open object storage: {e}");
            std::process::exit(1);
        }
    };

    info!("Connecting to database...");

    let pool = match create_pool(config.database_url.expose(), config.max_connections).await {
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
    if let Err(e) = migrator().run(&pool).await {
        error!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }
    info!("Database migrations completed");

    let state = AppState::new(pool, storage, config);
    let config = std::sync::Arc::clone(&state.config);
    let bind_addr = config.bind_addr();
    let app = build_app(state);

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to {}: {}", bind_addr, e);
            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!(
                    "Port {} is already in use. Try a different port with --port <PORT>",
                    config.port
                );
            } else if e.kind() == std::io::ErrorKind::AddrNotAvailable {
                error!(
                    "Cannot bind to host {}. Try --host 0.0.0.0 or --host 127.0.0.1",
                    config.host
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
