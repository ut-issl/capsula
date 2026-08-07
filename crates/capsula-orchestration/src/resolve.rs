use anyhow::Context;
use capsula_config::HeaderValueSource;
use std::collections::BTreeMap;
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

/// Check whether two URLs share the same origin (scheme, host, and effective port).
pub fn is_same_origin(a: &str, b: &str) -> anyhow::Result<bool> {
    let parse = |input: &str| {
        reqwest::Url::parse(input).with_context(|| format!("Invalid server URL: {input}"))
    };
    Ok(parse(a)?.origin() == parse(b)?.origin())
}

/// Resolve configured server headers into concrete `(name, value)` pairs.
///
/// Environment variables and commands are evaluated at call time, so dotenv
/// should be loaded before calling. Commands run with `project_root` as their
/// working directory, matching the `capture-command` hook.
pub fn resolve_server_headers(
    headers: &BTreeMap<String, HeaderValueSource>,
    project_root: &Path,
) -> anyhow::Result<Vec<(String, String)>> {
    headers
        .iter()
        .map(|(name, source)| {
            let value = resolve_header_value(name, source, project_root)?;
            debug!("Resolved server header: {}", name);
            Ok((name.clone(), value))
        })
        .collect()
}

fn resolve_header_value(
    name: &str,
    source: &HeaderValueSource,
    project_root: &Path,
) -> anyhow::Result<String> {
    match source {
        HeaderValueSource::Literal(value) => Ok(value.clone()),
        HeaderValueSource::Env(env_source) => {
            let value = std::env::var(&env_source.env).with_context(|| {
                format!(
                    "Header '{name}': environment variable '{}' is not set",
                    env_source.env
                )
            })?;
            Ok(match &env_source.prefix {
                Some(prefix) => format!("{prefix}{value}"),
                None => value,
            })
        }
        HeaderValueSource::Command(command_source) => {
            run_header_command(name, &command_source.command, project_root)
        }
    }
}

fn run_header_command(name: &str, command: &str, project_root: &Path) -> anyhow::Result<String> {
    let tokens = shlex::split(command)
        .ok_or_else(|| anyhow::anyhow!("Header '{name}': failed to parse command: {command}"))?;
    let [program, args @ ..] = tokens.as_slice() else {
        anyhow::bail!("Header '{name}': command is empty");
    };

    let output = std::process::Command::new(program)
        .args(args)
        .current_dir(project_root)
        .output()
        .with_context(|| format!("Header '{name}': failed to execute command '{command}'"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Header '{name}': command '{command}' failed with {}: {}",
            output.status,
            stderr.trim_end()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use capsula_config::{CommandHeaderSource, EnvHeaderSource};

    fn env_source(env: &str, prefix: Option<&str>) -> HeaderValueSource {
        HeaderValueSource::Env(EnvHeaderSource {
            env: env.to_string(),
            prefix: prefix.map(str::to_string),
        })
    }

    fn command_source(command: &str) -> HeaderValueSource {
        HeaderValueSource::Command(CommandHeaderSource {
            command: command.to_string(),
        })
    }

    #[test]
    fn resolves_literal_value_verbatim() {
        let headers = BTreeMap::from([(
            "x-static".to_string(),
            HeaderValueSource::Literal("fixed".to_string()),
        )]);

        let resolved = resolve_server_headers(&headers, Path::new(".")).unwrap();

        assert_eq!(
            resolved,
            vec![("x-static".to_string(), "fixed".to_string())]
        );
    }

    #[test]
    fn resolves_env_value_with_prefix() {
        // PATH is guaranteed to be set in test environments
        let path_value = std::env::var("PATH").unwrap();
        let headers = BTreeMap::from([(
            "authorization".to_string(),
            env_source("PATH", Some("Bearer ")),
        )]);

        let resolved = resolve_server_headers(&headers, Path::new(".")).unwrap();

        assert_eq!(resolved[0].1, format!("Bearer {path_value}"));
    }

    #[test]
    fn fails_when_env_var_is_missing() {
        let headers = BTreeMap::from([(
            "authorization".to_string(),
            env_source("CAPSULA_TEST_UNSET_VAR_1153", None),
        )]);

        let error = resolve_server_headers(&headers, Path::new(".")).unwrap_err();

        assert!(error.to_string().contains("CAPSULA_TEST_UNSET_VAR_1153"));
    }

    #[test]
    fn resolves_command_stdout_with_trailing_newline_trimmed() {
        let headers = BTreeMap::from([("x-token".to_string(), command_source("echo token-value"))]);

        let resolved = resolve_server_headers(&headers, Path::new(".")).unwrap();

        assert_eq!(
            resolved,
            vec![("x-token".to_string(), "token-value".to_string())]
        );
    }

    #[test]
    fn fails_with_stderr_when_command_exits_nonzero() {
        let headers = BTreeMap::from([(
            "x-token".to_string(),
            command_source("sh -c 'echo session-expired >&2; exit 1'"),
        )]);

        let error = resolve_server_headers(&headers, Path::new(".")).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("x-token"));
        assert!(message.contains("session-expired"));
    }

    #[test]
    fn runs_command_from_project_root() {
        let project_root = tempfile::tempdir().unwrap();
        std::fs::write(project_root.path().join("marker.txt"), "from-root\n").unwrap();
        let headers = BTreeMap::from([("x-token".to_string(), command_source("cat marker.txt"))]);

        let resolved = resolve_server_headers(&headers, project_root.path()).unwrap();

        assert_eq!(resolved[0].1, "from-root");
    }

    #[test]
    fn same_origin_ignores_paths_and_default_ports() {
        assert!(is_same_origin("http://capsula.example", "http://capsula.example:80/api").unwrap());
        assert!(
            is_same_origin("https://capsula.example:443/x", "https://capsula.example").unwrap()
        );
    }

    #[test]
    fn different_scheme_host_or_port_is_cross_origin() {
        assert!(!is_same_origin("http://capsula.example", "https://capsula.example").unwrap());
        assert!(!is_same_origin("https://a.example", "https://b.example").unwrap());
        assert!(
            !is_same_origin("https://capsula.example", "https://capsula.example:8443").unwrap()
        );
    }

    #[test]
    fn invalid_url_is_an_error_not_a_match() {
        assert!(is_same_origin("not a url", "https://capsula.example").is_err());
    }

    #[test]
    fn fails_when_command_is_not_found() {
        let headers = BTreeMap::from([(
            "x-token".to_string(),
            command_source("capsula-test-nonexistent-binary-1153"),
        )]);

        let error = resolve_server_headers(&headers, Path::new(".")).unwrap_err();

        assert!(error.to_string().contains("failed to execute"));
    }
}
