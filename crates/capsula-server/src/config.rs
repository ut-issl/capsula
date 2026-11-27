//! Server configuration from command-line flags and environment variables.
//!
//! Every option can be given as a flag or as the environment variable named in
//! its help text; flags take precedence. [`Config::validate`] checks the
//! combinations that clap cannot express, such as the identity-provider
//! settings required by [`AuthMode::Oidc`].

use clap::{Parser, ValueEnum};
use std::fmt;
use url::Url;

/// How callers are authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AuthMode {
    /// No authentication: every caller acts as the owner of the `local`
    /// organization. For development and tests on a loopback address only.
    Dev,
    /// Browser login with Google (OIDC) and/or GitHub (OAuth 2.0); API
    /// requests need a Bearer token or a session cookie.
    Oidc,
}

impl fmt::Display for AuthMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dev => f.write_str("dev"),
            Self::Oidc => f.write_str("oidc"),
        }
    }
}

/// A configuration value that must not appear in logs or debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// The secret itself. Keep the result out of logs.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::str::FromStr for SecretString {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// Settings of one OAuth 2.0 / OIDC client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: SecretString,
}

/// Web server for managing and viewing Capsula runs
#[derive(Parser, Debug, Clone)]
#[command(name = "capsula-server", version, about)]
pub struct Config {
    /// Host to bind to
    #[arg(short = 'H', long, env = "CAPSULA_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind to
    #[arg(short, long, env = "CAPSULA_PORT", default_value = "8500")]
    pub port: u16,

    /// Public base URL of this server, used for OAuth callbacks and absolute
    /// links (for example `https://capsula.example.com`). Required with
    /// `--auth-mode oidc`.
    #[arg(long, env = "CAPSULA_PUBLIC_URL")]
    pub public_url: Option<Url>,

    /// PostgreSQL connection string
    #[arg(short, long, env = "DATABASE_URL")]
    pub database_url: SecretString,

    /// Maximum database connections
    #[arg(long, env = "CAPSULA_MAX_CONNECTIONS", default_value = "5")]
    pub max_connections: u32,

    /// Object storage for captured files: `s3://bucket[/prefix]` (settings
    /// from the `AWS_*` environment variables) or `file:///path` (development
    /// and tests only)
    #[arg(long, env = "CAPSULA_STORAGE_URL")]
    pub storage_url: Url,

    /// How callers are authenticated
    #[arg(long, env = "CAPSULA_AUTH_MODE", value_enum)]
    pub auth_mode: AuthMode,

    /// OIDC issuer URL for browser login (for example `https://accounts.google.com`)
    #[arg(long, env = "CAPSULA_OIDC_ISSUER_URL", requires_all = ["oidc_client_id", "oidc_client_secret"])]
    pub oidc_issuer_url: Option<Url>,

    /// OIDC client id
    #[arg(long, env = "CAPSULA_OIDC_CLIENT_ID", requires_all = ["oidc_issuer_url", "oidc_client_secret"])]
    pub oidc_client_id: Option<String>,

    /// OIDC client secret
    #[arg(long, env = "CAPSULA_OIDC_CLIENT_SECRET", hide_env_values = true, requires_all = ["oidc_issuer_url", "oidc_client_id"])]
    pub oidc_client_secret: Option<SecretString>,

    /// GitHub OAuth App client id
    #[arg(
        long,
        env = "CAPSULA_GITHUB_CLIENT_ID",
        requires = "github_client_secret"
    )]
    pub github_client_id: Option<String>,

    /// GitHub OAuth App client secret
    #[arg(
        long,
        env = "CAPSULA_GITHUB_CLIENT_SECRET",
        hide_env_values = true,
        requires = "github_client_id"
    )]
    pub github_client_secret: Option<SecretString>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short, long, env = "RUST_LOG", default_value = "info")]
    pub log_level: String,

    /// Maximum body size for uploads in bytes (default: 100MB)
    #[arg(long, env = "CAPSULA_MAX_BODY_SIZE", default_value = "104857600")]
    pub max_body_size: usize,
}

/// A configuration that clap accepted but that cannot be used.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("--auth-mode oidc requires --public-url (CAPSULA_PUBLIC_URL)")]
    PublicUrlRequired,
    #[error(
        "--auth-mode oidc requires at least one identity provider: set the CAPSULA_OIDC_* or the CAPSULA_GITHUB_* variables"
    )]
    NoIdentityProvider,
    #[error("--public-url must be an http(s) URL without query or fragment, got `{0}`")]
    InvalidPublicUrl(Url),
}

impl Config {
    /// Check the option combinations that clap cannot express.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(public_url) = &self.public_url {
            let is_http = matches!(public_url.scheme(), "http" | "https");
            if !is_http
                || public_url.host_str().is_none()
                || public_url.query().is_some()
                || public_url.fragment().is_some()
            {
                return Err(ConfigError::InvalidPublicUrl(public_url.clone()));
            }
        }
        if self.auth_mode == AuthMode::Oidc {
            if self.public_url.is_none() {
                return Err(ConfigError::PublicUrlRequired);
            }
            if self.oidc().is_none() && self.github().is_none() {
                return Err(ConfigError::NoIdentityProvider);
            }
        }
        Ok(())
    }

    /// The OIDC provider, when fully configured.
    #[must_use]
    pub fn oidc(&self) -> Option<(&Url, OAuthClient)> {
        Some((
            self.oidc_issuer_url.as_ref()?,
            OAuthClient {
                client_id: self.oidc_client_id.clone()?,
                client_secret: self.oidc_client_secret.clone()?,
            },
        ))
    }

    /// The GitHub OAuth App, when fully configured.
    #[must_use]
    pub fn github(&self) -> Option<OAuthClient> {
        Some(OAuthClient {
            client_id: self.github_client_id.clone()?,
            client_secret: self.github_client_secret.clone()?,
        })
    }

    /// The `host:port` to bind to.
    #[must_use]
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Config, clap::Error> {
        // Clear the process environment from the parse: every option is given explicitly.
        let base = [
            "capsula-server",
            "--database-url",
            "postgres://u:p@localhost/db",
            "--storage-url",
            "file:///tmp/capsula",
        ];
        Config::try_parse_from(base.iter().chain(args))
    }

    #[test]
    fn minimal_dev_config_parses_and_validates() {
        let config = parse(&["--auth-mode", "dev"]).unwrap();
        assert_eq!(config.auth_mode, AuthMode::Dev);
        assert_eq!(config.bind_addr(), "127.0.0.1:8500");
        assert_eq!(config.storage_url.scheme(), "file");
        assert_eq!(config.database_url.expose(), "postgres://u:p@localhost/db");
        assert!(config.public_url.is_none());
        assert!(config.oidc().is_none());
        assert!(config.github().is_none());
        config.validate().unwrap();
    }

    #[test]
    fn auth_mode_and_storage_url_are_required() {
        assert!(parse(&[]).is_err());
        assert!(
            Config::try_parse_from([
                "capsula-server",
                "--database-url",
                "postgres://localhost/db",
                "--auth-mode",
                "dev",
            ])
            .is_err()
        );
        assert!(parse(&["--auth-mode", "public"]).is_err());
    }

    #[test]
    fn oidc_mode_needs_public_url_and_a_provider() {
        let config = parse(&["--auth-mode", "oidc"]).unwrap();
        assert_eq!(config.validate(), Err(ConfigError::PublicUrlRequired));

        let config = parse(&[
            "--auth-mode",
            "oidc",
            "--public-url",
            "https://capsula.example.com",
        ])
        .unwrap();
        assert_eq!(config.validate(), Err(ConfigError::NoIdentityProvider));

        let config = parse(&[
            "--auth-mode",
            "oidc",
            "--public-url",
            "https://capsula.example.com",
            "--github-client-id",
            "id",
            "--github-client-secret",
            "secret",
        ])
        .unwrap();
        config.validate().unwrap();
        let github = config.github().unwrap();
        assert_eq!(github.client_id, "id");
        assert_eq!(github.client_secret.expose(), "secret");
        assert!(config.oidc().is_none());
    }

    #[test]
    fn oidc_provider_is_all_or_nothing() {
        assert!(parse(&["--auth-mode", "dev", "--oidc-client-id", "id"]).is_err());
        assert!(parse(&["--auth-mode", "dev", "--github-client-secret", "s"]).is_err());

        let config = parse(&[
            "--auth-mode",
            "dev",
            "--oidc-issuer-url",
            "https://accounts.google.com",
            "--oidc-client-id",
            "id",
            "--oidc-client-secret",
            "secret",
        ])
        .unwrap();
        let (issuer, client) = config.oidc().unwrap();
        assert_eq!(issuer.as_str(), "https://accounts.google.com/");
        assert_eq!(client.client_id, "id");
        config.validate().unwrap();
    }

    #[test]
    fn public_url_must_be_a_plain_http_origin() {
        for bad in [
            "ftp://x.example",
            "https://x.example/?a=b",
            "https://x.example/#f",
        ] {
            let config = parse(&["--auth-mode", "dev", "--public-url", bad]).unwrap();
            assert!(
                matches!(config.validate(), Err(ConfigError::InvalidPublicUrl(_))),
                "{bad}"
            );
        }
        let config = parse(&[
            "--auth-mode",
            "dev",
            "--public-url",
            "http://localhost:8500/capsula",
        ])
        .unwrap();
        config.validate().unwrap();
    }

    #[test]
    fn secrets_are_redacted_in_debug_output() {
        let config = parse(&[
            "--auth-mode",
            "dev",
            "--github-client-id",
            "id",
            "--github-client-secret",
            "hunter2",
        ])
        .unwrap();
        let debug = format!("{config:?}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(!debug.contains("postgres://u:p@"), "{debug}");
        assert!(debug.contains("[redacted]"));
    }
}
