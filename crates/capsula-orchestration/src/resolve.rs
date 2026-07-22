use std::path::{Path, PathBuf};
use tracing::debug;

/// Resolve the vault path with priority: explicit override > environment variable > config file.
///
/// The `override_path` parameter corresponds to CLI arguments or other explicit overrides.
/// Environment variables are read at call time, so dotenv should be loaded before calling.
pub(crate) fn resolve_vault_path(
    override_path: Option<PathBuf>,
    config_vault_path: &Path,
    project_root: &Path,
) -> PathBuf {
    if let Some(path) = override_path {
        debug!("Using vault path from override: {}", path.display());
        return if path.is_absolute() {
            path
        } else {
            project_root.join(path)
        };
    }

    if let Ok(env_path) = std::env::var("CAPSULA_VAULT_PATH") {
        let path = PathBuf::from(&env_path);
        debug!(
            "Using vault path from CAPSULA_VAULT_PATH env var: {}",
            path.display()
        );
        return if path.is_absolute() {
            path
        } else {
            project_root.join(path)
        };
    }

    debug!(
        "Using vault path from config file: {}",
        config_vault_path.display()
    );
    if config_vault_path.is_absolute() {
        config_vault_path.to_path_buf()
    } else {
        project_root.join(config_vault_path)
    }
}

/// Resolve the server URL with priority: explicit override > environment variable > config file.
///
/// The `override_url` parameter corresponds to CLI arguments or other explicit overrides.
/// Environment variables are read at call time, so dotenv should be loaded before calling.
pub fn resolve_server_url(
    override_url: Option<String>,
    config_server: Option<&str>,
) -> Option<String> {
    if let Some(server) = override_url {
        debug!("Using server URL from override: {}", server);
        return Some(server);
    }

    if let Ok(env_server) = std::env::var("CAPSULA_SERVER_URL") {
        debug!(
            "Using server URL from CAPSULA_SERVER_URL env var: {}",
            env_server
        );
        return Some(env_server);
    }

    if let Some(server) = config_server {
        debug!("Using server URL from config file: {}", server);
        return Some(server.to_string());
    }

    None
}
