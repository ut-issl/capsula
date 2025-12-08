use capsula_core::{
    error::{CapsulaError, CapsulaResult},
    hook::PhaseMarker,
};
use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Invalid configuration: {message}")]
    Invalid { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

/// Convert `ConfigError` to `CapsulaError` for cross-crate compatibility
impl From<ConfigError> for CapsulaError {
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::TomlParse(e) => CapsulaError::Configuration {
                message: format!("Failed to parse TOML configuration: {e}"),
            },
            ConfigError::FileNotFound { path } => CapsulaError::Configuration {
                message: format!(
                    "Configuration file not found at '{}'. Create a 'capsula.toml' file or specify a custom path with --config",
                    path.display()
                ),
            },
            ConfigError::Invalid { message } => CapsulaError::Configuration { message },
            ConfigError::Io(e) => CapsulaError::from(e),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct CapsulaConfig {
    pub vault: VaultConfig,
    #[serde(default)]
    pub pre_run: HookPhaseConfig,
    #[serde(default)]
    pub post_run: HookPhaseConfig,
}

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub name: String,
    pub path: PathBuf,
}

impl<'de> Deserialize<'de> for VaultConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VaultConfigHelper {
            name: String,
            path: Option<PathBuf>,
        }

        let helper = VaultConfigHelper::deserialize(deserializer)?;
        let path = helper
            .path
            .unwrap_or_else(|| PathBuf::from(format!(".capsula/{}", helper.name)));

        Ok(VaultConfig {
            name: helper.name,
            path,
        })
    }
}

/// A phase configuration that contains hooks
#[derive(Deserialize, Debug, Clone, Default)]
pub struct HookPhaseConfig {
    #[serde(default)]
    pub hooks: Vec<HookEnvelope>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HookEnvelope {
    pub id: String,
    #[serde(flatten)]
    pub rest: serde_json::Value,
}

impl CapsulaConfig {
    pub fn from_toml_str(content: &str) -> ConfigResult<Self> {
        Ok(toml::from_str(content)?)
    }

    pub fn from_file(path: impl AsRef<std::path::Path>) -> ConfigResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ConfigError::FileNotFound {
                    path: path.to_path_buf(),
                }
            } else {
                ConfigError::Io(e)
            }
        })?;
        Self::from_toml_str(&content)
    }
}

/// Build hooks from any phase config that contains hooks
pub fn build_hooks<P: PhaseMarker>(
    phase: &HookPhaseConfig,
    project_root: &Path,
    registry: &capsula_registry::HookRegistry<P>,
) -> CapsulaResult<Vec<Box<dyn capsula_core::hook::HookErased<P>>>> {
    phase
        .hooks
        .iter()
        .map(|envelope| registry.create_hook(&envelope.id, &envelope.rest, project_root))
        .collect()
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_config() {
        let config_str = r#"
[vault]
name = "capsula"

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

[[pre-run.hooks]]
id = "capture-file"
path = "capsula.toml"
copy = true
hash = true

[[pre-run.hooks]]
id = "capture-file"
path = "Cargo.toml"
hash = true

[[post-run.hooks]]
id = "capture-env"
key = "PATH"
"#;

        let config = CapsulaConfig::from_toml_str(config_str).unwrap();

        assert_eq!(config.vault.name, "capsula");
        assert_eq!(config.vault.path, PathBuf::from(".capsula/capsula"));

        assert_eq!(config.pre_run.hooks.len(), 4);
        assert_eq!(config.pre_run.hooks[0].id, "capture-cwd");
        assert_eq!(config.pre_run.hooks[1].id, "capture-git-repo");
        assert_eq!(config.pre_run.hooks[2].id, "capture-file");
        assert_eq!(config.pre_run.hooks[3].id, "capture-file");

        assert_eq!(config.post_run.hooks.len(), 1);
        assert_eq!(config.post_run.hooks[0].id, "capture-env");
    }

    #[test]
    fn test_vault_config_with_explicit_path() {
        let config_str = r#"
[vault]
name = "my_vault"
path = "/custom/path/to/vault"

[[pre-run.hooks]]
id = "capture-cwd"
"#;

        let config = CapsulaConfig::from_toml_str(config_str).unwrap();

        assert_eq!(config.vault.name, "my_vault");
        assert_eq!(config.vault.path, PathBuf::from("/custom/path/to/vault"));
    }

    #[test]
    fn test_vault_config_without_path() {
        let config_str = r#"
[vault]
name = "test_vault"

[[pre-run.hooks]]
id = "capture-cwd"
"#;

        let config = CapsulaConfig::from_toml_str(config_str).unwrap();

        assert_eq!(config.vault.name, "test_vault");
        assert_eq!(config.vault.path, PathBuf::from(".capsula/test_vault"));
    }
}
