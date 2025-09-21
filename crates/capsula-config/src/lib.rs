use capsula_core::error::{CapsulaError, CoreResult};
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

/// Convert ConfigError to CoreError for cross-crate compatibility
impl From<ConfigError> for CapsulaError {
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::TomlParse(e) => CapsulaError::Configuration {
                message: format!("Failed to parse TOML configuration: {}", e),
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
pub struct CapsulaConfig {
    pub vault: VaultConfig,
    pub phase: PhaseConfig,
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

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PhaseConfig {
    #[serde(default)]
    pub pre: PrePhaseConfig,
    #[serde(rename = "in", default)]
    pub in_phase: InPhaseConfig,
    #[serde(default)]
    pub post: PostPhaseConfig,
}

/// A phase configuration that contains contexts
#[derive(Deserialize, Debug, Clone, Default)]
pub struct ContextPhaseConfig {
    #[serde(default)]
    pub contexts: Vec<ContextEnvelope>,
}

/// A phase configuration that contains watchers
#[derive(Deserialize, Debug, Clone, Default)]
pub struct WatcherPhaseConfig {
    #[serde(default)]
    pub watchers: Vec<WatcherSpec>,
}

// Type aliases for semantic clarity
pub type PrePhaseConfig = ContextPhaseConfig;
pub type PostPhaseConfig = ContextPhaseConfig;
pub type InPhaseConfig = WatcherPhaseConfig;

#[derive(Deserialize, Debug, Clone)]
pub struct ContextEnvelope {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(flatten)]
    pub rest: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WatcherSpec {
    Time(TimeWatcherSpec),
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct TimeWatcherSpec {}

impl CapsulaConfig {
    pub fn from_str(content: &str) -> ConfigResult<Self> {
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
        Self::from_str(&content)
    }
}

/// Build contexts from any phase config that contains contexts
pub fn build_contexts(
    phase: &ContextPhaseConfig,
    project_root: &Path,
    registry: &capsula_registry::ContextRegistry,
) -> CoreResult<Vec<Box<dyn capsula_core::context::ContextErased>>> {
    phase
        .contexts
        .iter()
        .map(|envelope| registry.create_context(&envelope.ty, &envelope.rest, project_root))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_config() {
        let config_str = r#"
[vault]
name = "capsula"

[[phase.pre.contexts]]
type = "cwd"

[[phase.pre.contexts]]
type = "git"
path = "."

[[phase.pre.contexts]]
type = "file"
path = "capsula.toml"
copy = true
hash = true

[[phase.pre.contexts]]
type = "file"
path = "Cargo.toml"
hash = true

[[phase.in.watchers]]
type = "time"

[[phase.post.contexts]]
type = "env"
key = "PATH"
"#;

        let config = CapsulaConfig::from_str(config_str).unwrap();

        assert_eq!(config.vault.name, "capsula");
        assert_eq!(config.vault.path, PathBuf::from(".capsula/capsula"));

        assert_eq!(config.phase.pre.contexts.len(), 4);
        assert_eq!(config.phase.pre.contexts[0].ty, "cwd");
        assert_eq!(config.phase.pre.contexts[1].ty, "git");
        assert_eq!(config.phase.pre.contexts[2].ty, "file");
        assert_eq!(config.phase.pre.contexts[3].ty, "file");

        assert_eq!(config.phase.in_phase.watchers.len(), 1);
        assert!(matches!(
            &config.phase.in_phase.watchers[0],
            WatcherSpec::Time(_)
        ));

        assert_eq!(config.phase.post.contexts.len(), 1);
        assert_eq!(config.phase.post.contexts[0].ty, "env");
    }

    #[test]
    fn test_vault_config_with_explicit_path() {
        let config_str = r#"
[vault]
name = "my_vault"
path = "/custom/path/to/vault"

[[phase.pre.contexts]]
type = "cwd"
"#;

        let config = CapsulaConfig::from_str(config_str).unwrap();

        assert_eq!(config.vault.name, "my_vault");
        assert_eq!(config.vault.path, PathBuf::from("/custom/path/to/vault"));
    }

    #[test]
    fn test_vault_config_without_path() {
        let config_str = r#"
[vault]
name = "test_vault"

[[phase.pre.contexts]]
type = "cwd"
"#;

        let config = CapsulaConfig::from_str(config_str).unwrap();

        assert_eq!(config.vault.name, "test_vault");
        assert_eq!(config.vault.path, PathBuf::from(".capsula/test_vault"));
    }
}
