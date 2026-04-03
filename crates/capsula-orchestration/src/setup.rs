use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use capsula_config::CapsulaConfig;
use tracing::{debug, info, warn};

use crate::resolve::resolve_vault_path;

/// Result of loading and resolving a capsula configuration.
pub struct LoadedConfig {
    pub config: CapsulaConfig,
    pub project_root: PathBuf,
    pub vault_dir: PathBuf,
}

/// Load a capsula configuration file, resolve dotenv and vault path.
///
/// `config_path` is the path to `capsula.toml`.
/// `vault_path_override` is an optional CLI override for the vault path.
pub fn load_config(
    config_path: &Path,
    vault_path_override: Option<PathBuf>,
) -> Result<LoadedConfig> {
    if !config_path.exists() {
        anyhow::bail!(
            "Configuration file not found at '{}'",
            config_path.display()
        );
    }

    let config_path = config_path
        .canonicalize()
        .with_context(|| format!("Failed to resolve config path: {}", config_path.display()))?;

    let project_root = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine project root from config file"))?
        .to_path_buf();

    debug!("Loading configuration from: {}", config_path.display());
    let config = CapsulaConfig::from_file(&config_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_path.display()
        )
    })?;

    // Load dotenv file if specified
    if let Some(dotenv_path) = &config.dotenv {
        let dotenv_full_path = if dotenv_path.is_absolute() {
            dotenv_path.clone()
        } else {
            project_root.join(dotenv_path)
        };

        debug!("Loading dotenv file from: {}", dotenv_full_path.display());

        match dotenvy::from_path(&dotenv_full_path) {
            Ok(()) => {
                info!(
                    "Loaded environment variables from: {}",
                    dotenv_full_path.display()
                );
            }
            Err(e) => {
                warn!(
                    "Failed to load dotenv file from {}: {}",
                    dotenv_full_path.display(),
                    e
                );
            }
        }
    }

    let vault_dir = resolve_vault_path(vault_path_override, &config.vault.path, &project_root);

    Ok(LoadedConfig {
        config,
        project_root,
        vault_dir,
    })
}
