use capsula_core::{
    error::{CapsulaError, CapsulaResult},
    hook::PhaseMarker,
};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

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
            ConfigError::TomlParse(e) => Self::Configuration {
                message: format!("Failed to parse TOML configuration: {e}"),
            },
            ConfigError::FileNotFound { path } => Self::Configuration {
                message: format!(
                    "Configuration file not found at '{}'. Create a 'capsula.toml' file or specify a custom path with --config",
                    path.display()
                ),
            },
            ConfigError::Invalid { message } => Self::Configuration { message },
            ConfigError::Io(e) => Self::from(e),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct CapsulaConfig {
    pub vault: VaultConfig,
    #[serde(default)]
    pub dotenv: Option<PathBuf>,
    #[serde(default)]
    pub server: Option<ServerConfig>,
    #[serde(default)]
    pub pre_run: HookPhaseConfig,
    #[serde(default)]
    pub post_run: HookPhaseConfig,
}

/// Server connection settings.
///
/// Accepts either a plain URL string (`server = "https://..."`) or a table
/// with a `url` and optional `headers` attached to every request:
///
/// ```toml
/// [server]
/// url = "https://capsula.example.com"
///
/// [server.headers]
/// Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }
/// ```
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, HeaderValueSource>,
}

impl<'de> Deserialize<'de> for ServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct DetailedServerConfigHelper {
            url: String,
            #[serde(default)]
            headers: BTreeMap<String, HeaderValueSource>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ServerConfigHelper {
            Url(String),
            Detailed(DetailedServerConfigHelper),
        }

        Ok(match ServerConfigHelper::deserialize(deserializer)? {
            ServerConfigHelper::Url(url) => Self {
                url,
                headers: BTreeMap::new(),
            },
            ServerConfigHelper::Detailed(detailed) => Self {
                url: detailed.url,
                headers: detailed.headers,
            },
        })
    }
}

/// Source of an HTTP header value sent with server requests.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum HeaderValueSource {
    /// The string is used as the header value verbatim.
    Literal(String),
    /// The value is read from an environment variable.
    Env(EnvHeaderSource),
    /// The value is the trimmed stdout of a command.
    Command(CommandHeaderSource),
}

/// Header value read from an environment variable.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvHeaderSource {
    pub env: String,
    /// Prepended to the variable's value (e.g. `"Bearer "`).
    #[serde(default)]
    pub prefix: Option<String>,
}

/// Header value produced by running a command and capturing its stdout.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommandHeaderSource {
    pub command: String,
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

        Ok(Self {
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
    rest: serde_json::Value,
}

impl CapsulaConfig {
    fn from_toml_str(content: &str) -> ConfigResult<Self> {
        debug!("Parsing TOML configuration ({} bytes)", content.len());
        let config = toml::from_str(content)?;
        debug!("TOML configuration parsed successfully");
        Ok(config)
    }

    pub fn from_file(path: impl AsRef<std::path::Path>) -> ConfigResult<Self> {
        let path = path.as_ref();
        debug!("Reading configuration file: {}", path.display());
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
    debug!(
        "Building {} hooks from phase configuration",
        phase.hooks.len()
    );
    let hooks: Vec<_> = phase
        .hooks
        .iter()
        .map(|envelope| {
            debug!("Creating hook: {}", envelope.id);
            registry.create_hook(&envelope.id, &envelope.rest, project_root)
        })
        .collect::<Result<_, _>>()?;
    debug!("Successfully created {} hook instances", hooks.len());
    Ok(hooks)
}

#[cfg(test)]
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
glob = "capsula.toml"
mode = "copy"
hash = "sha256"

[[pre-run.hooks]]
id = "capture-file"
glob = "Cargo.toml"
hash = "sha256"

[[post-run.hooks]]
id = "capture-env"
name = "PATH"
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
    fn parses_server_as_plain_url_string() {
        let config_str = r#"
server = "https://capsula.example.com"

[vault]
name = "v"
"#;

        let config = CapsulaConfig::from_toml_str(config_str).unwrap();

        let server = config.server.unwrap();
        assert_eq!(server.url, "https://capsula.example.com");
        assert!(server.headers.is_empty());
    }

    #[test]
    fn server_defaults_to_none_when_absent() {
        let config_str = r#"
[vault]
name = "v"
"#;

        let config = CapsulaConfig::from_toml_str(config_str).unwrap();

        assert!(config.server.is_none());
    }

    #[test]
    fn parses_server_table_with_all_header_source_kinds() {
        let config_str = r#"
[vault]
name = "v"

[server]
url = "https://capsula.example.com"

[server.headers]
x-literal = "fixed-value"
Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }
cf-access-token = { command = "cloudflared access token --app=https://capsula.example.com" }
CF-Access-Client-Id = { env = "CF_ACCESS_CLIENT_ID" }
"#;

        let config = CapsulaConfig::from_toml_str(config_str).unwrap();

        let server = config.server.unwrap();
        assert_eq!(server.url, "https://capsula.example.com");
        assert_eq!(
            server.headers["x-literal"],
            HeaderValueSource::Literal("fixed-value".to_string())
        );
        assert_eq!(
            server.headers["Authorization"],
            HeaderValueSource::Env(EnvHeaderSource {
                env: "CAPSULA_TOKEN".to_string(),
                prefix: Some("Bearer ".to_string()),
            })
        );
        assert_eq!(
            server.headers["cf-access-token"],
            HeaderValueSource::Command(CommandHeaderSource {
                command: "cloudflared access token --app=https://capsula.example.com".to_string(),
            })
        );
        assert_eq!(
            server.headers["CF-Access-Client-Id"],
            HeaderValueSource::Env(EnvHeaderSource {
                env: "CF_ACCESS_CLIENT_ID".to_string(),
                prefix: None,
            })
        );
    }

    #[test]
    fn rejects_header_source_mixing_env_and_command() {
        let config_str = r#"
[vault]
name = "v"

[server]
url = "https://capsula.example.com"

[server.headers]
Authorization = { env = "CAPSULA_TOKEN", command = "echo x" }
"#;

        assert!(CapsulaConfig::from_toml_str(config_str).is_err());
    }

    #[test]
    fn rejects_unknown_field_in_server_table() {
        let config_str = r#"
[vault]
name = "v"

[server]
url = "https://capsula.example.com"

[server.header]
Authorization = "typo-should-fail"
"#;

        assert!(CapsulaConfig::from_toml_str(config_str).is_err());
    }

    #[test]
    fn rejects_server_table_without_url() {
        let config_str = r#"
[vault]
name = "v"

[server]
headers = {}
"#;

        assert!(CapsulaConfig::from_toml_str(config_str).is_err());
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
