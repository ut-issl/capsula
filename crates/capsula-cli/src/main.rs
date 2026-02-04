//! Capsula CLI main entry point
#![allow(
    clippy::print_stdout,
    reason = "Printing is acceptable in main CLI code"
)]

use anyhow::{Context, Result};
use capsula_config::{CapsulaConfig, HookPhaseConfig};
use capsula_core::hook::{PhaseMarker, PostRun, PreRun, RuntimeParams};
use capsula_core::run::{PreparedRun, Run};
use chrono::DateTime;
use clap::{Parser, Subcommand};
use names::Generator;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};
use ulid::Ulid;

const VERSION: &str = if env!("GIT_HASH").is_empty() {
    env!("CARGO_PKG_VERSION")
} else {
    concat!(
        env!("CARGO_PKG_VERSION"),
        " (commit: ",
        env!("GIT_HASH"),
        ")"
    )
};

#[derive(Parser, Debug)]
#[command(name = "capsula", bin_name = "capsula", version = VERSION, about = "Capsula CLI")]
struct Cli {
    #[arg(short, long, global(true))]
    config: Option<PathBuf>,

    /// Override the vault path (can also be set via `CAPSULA_VAULT_PATH` env var after dotenv loading)
    #[arg(long, global(true))]
    vault_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    /// Print the run directory for a given run name
    RunDir {
        /// Run name to locate (e.g., happy-river)
        run_name: String,
    },
    List,
    Push {
        /// Run ID or name to push (e.g., 01HQXYZ... or chubby-back)
        run_id: Option<String>,

        /// Push all runs in the vault
        #[arg(long, conflicts_with = "run_id")]
        all: bool,

        /// Server URL (can also be set via `CAPSULA_SERVER_URL` env var after dotenv loading)
        #[arg(long)]
        server: Option<String>,
    },
    Vaults {
        #[command(subcommand)]
        command: VaultsCommands,
    },
}

#[derive(Subcommand, Debug)]
enum VaultsCommands {
    List {
        /// Server URL (can also be set via `CAPSULA_SERVER_URL` env var after dotenv loading)
        #[arg(long)]
        server: Option<String>,
    },
}

/// Resolve the vault path with priority: CLI argument > environment variable > config file.
///
/// This function is called after dotenv loading, so environment variables from the dotenv file
/// are available. This is important because Clap's `env` attribute reads environment variables
/// at parse time, before the dotenv file is loaded.
fn resolve_vault_path(
    cli_vault_path: Option<PathBuf>,
    config_vault_path: &std::path::Path,
    project_root: &std::path::Path,
) -> PathBuf {
    // Priority 1: CLI argument
    if let Some(path) = cli_vault_path {
        debug!("Using vault path from CLI argument: {}", path.display());
        return if path.is_absolute() {
            path
        } else {
            project_root.join(path)
        };
    }

    // Priority 2: Environment variable (checked after dotenv loading)
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

    // Priority 3: Config file
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

/// Resolve the server URL with priority: CLI argument > environment variable > config file.
///
/// This function is called after dotenv loading, so environment variables from the dotenv file
/// are available. This is important because Clap's `env` attribute reads environment variables
/// at parse time, before the dotenv file is loaded.
fn resolve_server_url(cli_server: Option<String>, config_server: Option<&str>) -> Option<String> {
    // Priority 1: CLI argument
    if let Some(server) = cli_server {
        debug!("Using server URL from CLI argument: {}", server);
        return Some(server);
    }

    // Priority 2: Environment variable (checked after dotenv loading)
    if let Ok(env_server) = std::env::var("CAPSULA_SERVER_URL") {
        debug!(
            "Using server URL from CAPSULA_SERVER_URL env var: {}",
            env_server
        );
        return Some(env_server);
    }

    // Priority 3: Config file
    if let Some(server) = config_server {
        debug!("Using server URL from config file: {}", server);
        return Some(server.to_string());
    }

    None
}

fn create_pre_run_hook_registry() -> capsula_registry::HookRegistry<PreRun> {
    // Use the standard registry with all built-in hook types
    capsula_registry::standard_pre_run_hook_registry()
}

fn create_post_run_hook_registry() -> capsula_registry::HookRegistry<PostRun> {
    // Use the standard registry with all built-in hook types
    capsula_registry::standard_post_run_hook_registry()
}

fn build_and_run_hooks<P: PhaseMarker>(
    run_metadata: &PreparedRun,
    runtime_params: &RuntimeParams<P>,
    hook_phase_config: &HookPhaseConfig,
    hook_registry: &capsula_registry::HookRegistry<P>,
    project_root: &std::path::Path,
) -> Result<(Vec<serde_json::Value>, bool)> {
    debug!(
        "Building {} hooks from configuration",
        hook_phase_config.hooks.len()
    );
    let hooks = capsula_config::build_hooks(hook_phase_config, project_root, hook_registry)
        .context("Failed to build hooks from configuration")?;
    debug!("Successfully built {} hook instances", hooks.len());

    let results: Vec<_> = hooks
        .iter()
        .enumerate()
        .map(|(idx, hook)| {
            let hook_identifier = hook_phase_config.hooks.get(idx).map_or_else(
                || format!("hook[{idx}]"),
                |config_hook| config_hook.id.clone(),
            );

            let hook_config_json = hook
                .config_as_json()
                .unwrap_or_else(|_| json!({ "__error": "Failed to serialize hook config" }));

            debug!("Running hook: {}", hook_identifier);
            match hook.run(run_metadata, runtime_params) {
                Ok(captured) => {
                    debug!("Hook '{}' completed successfully", hook_identifier);
                    let should_abort = captured.abort_requested();

                    // Convert to JSON and add metadata object
                    let mut json = captured.serialize_json().unwrap_or_else(
                        |_| json!({ "__error": "Failed to serialize captured data" }),
                    );
                    if let serde_json::Value::Object(ref mut map) = json {
                        let metadata = json!({
                            "id": hook.id(),
                            "config": hook_config_json,
                            "success": true,
                        });
                        map.insert("__meta".to_string(), metadata);
                    }
                    (json, should_abort)
                }
                Err(e) => {
                    let error = anyhow::anyhow!(e);
                    error!("Failed to run {hook_identifier} (config index {idx}): {error:#}");
                    // Only include the metadata with error information
                    let json = json!({
                        "__meta": json!({
                            "config": hook_config_json,
                            "success": false,
                            "error": format!("{}", error)
                        })}
                    );
                    (json, false) // Do not abort on capture failure
                }
            }
        })
        .collect();

    let json_results = results.iter().map(|(json, _)| json.clone()).collect();
    let should_abort = results.iter().any(|(_, abort)| *abort);

    Ok((json_results, should_abort))
}

#[derive(Debug, Deserialize)]
struct RunMetadata {
    name: String,
    command: Vec<String>,
    timestamp: String,
}

fn list_runs(vault_dir: &std::path::Path) -> Result<Vec<RunMetadata>> {
    let mut runs = Vec::new();

    // Check if vault directory exists
    if !vault_dir.exists() {
        return Ok(runs);
    }

    // Iterate through date directories
    for date_entry in std::fs::read_dir(vault_dir)
        .with_context(|| format!("Failed to read vault directory: {}", vault_dir.display()))?
    {
        let date_entry = date_entry?;
        let date_path = date_entry.path();

        // Skip if not a directory or is .gitignore
        if !date_path.is_dir() {
            continue;
        }

        // Iterate through run directories within each date
        for run_entry in std::fs::read_dir(&date_path)
            .with_context(|| format!("Failed to read date directory: {}", date_path.display()))?
        {
            let run_entry = run_entry?;
            let run_path = run_entry.path();

            // Skip if not a directory
            if !run_path.is_dir() {
                continue;
            }

            // Look for _capsula/metadata.json
            let metadata_path = run_path.join("_capsula").join("metadata.json");
            if metadata_path.exists() {
                // Read and parse metadata
                match std::fs::read_to_string(&metadata_path) {
                    Ok(content) => match serde_json::from_str::<RunMetadata>(&content) {
                        Ok(metadata) => runs.push(metadata),
                        Err(e) => {
                            warn!(
                                "Failed to parse metadata from {}: {}",
                                metadata_path.display(),
                                e
                            );
                        }
                    },
                    Err(e) => {
                        warn!(
                            "Failed to read metadata from {}: {}",
                            metadata_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    runs.sort_by(|a, b| {
        // Parse timestamps and compare
        let time_a = DateTime::parse_from_rfc3339(&a.timestamp).ok();
        let time_b = DateTime::parse_from_rfc3339(&b.timestamp).ok();

        match (time_a, time_b) {
            (Some(a), Some(b)) => b.cmp(&a), // Reverse order for newest first
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    Ok(runs)
}

fn push_single_run(
    run_dir: &std::path::Path,
    vault_name: &str,
    server_url: &str,
    client: &capsula_client::CapsulaClient,
) -> Result<()> {
    let capsula_dir = run_dir.join("_capsula");

    // Read metadata
    let metadata_path = capsula_dir.join("metadata.json");
    let metadata_content = std::fs::read_to_string(&metadata_path)
        .with_context(|| format!("Failed to read metadata from {}", metadata_path.display()))?;
    let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

    let run_name = metadata["name"].as_str().unwrap_or("unknown");
    info!("Pushing run: {}", run_name);

    // Read pre-run and post-run hooks
    let pre_run_path = capsula_dir.join("pre-run.json");
    let pre_run_hooks = if pre_run_path.exists() {
        let content = std::fs::read_to_string(&pre_run_path)?;
        Some(serde_json::from_str::<Vec<serde_json::Value>>(&content)?)
    } else {
        None
    };

    let post_run_path = capsula_dir.join("post-run.json");
    let post_run_hooks = if post_run_path.exists() {
        let content = std::fs::read_to_string(&post_run_path)?;
        Some(serde_json::from_str::<Vec<serde_json::Value>>(&content)?)
    } else {
        None
    };

    // Read command output
    let command_json_path = capsula_dir.join("command.json");
    let command_output = if command_json_path.exists() {
        let content = std::fs::read_to_string(&command_json_path)?;
        serde_json::from_str::<serde_json::Value>(&content)?
    } else {
        serde_json::json!({})
    };

    // Convert duration object to milliseconds
    let duration_ms = command_output.get("duration").and_then(|d| {
        let secs = d.get("secs")?.as_u64()?;
        let nanos = d.get("nanos")?.as_u64()?;
        Some((secs * 1000) + (nanos / 1_000_000))
    });

    let create_run_payload = serde_json::json!({
        "id": metadata["id"],
        "name": metadata["name"],
        "timestamp": metadata["timestamp"],
        "command": serde_json::to_string(&metadata["command"])?,
        "vault": vault_name,
        "project_root": metadata["project_root"],
        "exit_code": command_output.get("exit_code"),
        "duration_ms": duration_ms,
        "stdout": command_output.get("stdout"),
        "stderr": command_output.get("stderr"),
    });

    // Post the run metadata
    let url = format!("{server_url}/api/v1/runs");
    let http_client = reqwest::blocking::Client::new();
    let response = http_client.post(&url).json(&create_run_payload).send()?;

    if !response.status().is_success() {
        if response.status().as_u16() == 409 {
            // Return a special error that caller can detect
            anyhow::bail!("Run already exists: {run_name}");
        }
        anyhow::bail!("Failed to create run on server: {}", response.status());
    }

    // Collect files to upload
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(run_dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "_capsula")
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let local_path = entry.path();
            let relative_path = local_path.strip_prefix(run_dir)?;
            files.push((local_path.to_path_buf(), relative_path.to_path_buf()));
        }
    }

    // Upload files and hooks
    if !files.is_empty() || pre_run_hooks.is_some() || post_run_hooks.is_some() {
        let actual_run_id = metadata["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Run ID not found in metadata"))?;
        let response = client.upload_run(actual_run_id, &files, pre_run_hooks, post_run_hooks)?;
        debug!(
            "Upload complete: {} files, {} bytes",
            response.files_processed, response.total_bytes
        );
    }

    info!("✓ Pushed: {}", run_name);
    Ok(())
}

fn find_run_dir(vault_dir: &std::path::Path, run_id: &str) -> Result<PathBuf> {
    // Search for the run directory by ID or name
    // Structure: vault_dir/{YYYY-MM-DD}/{HHMMSS-name}/
    for entry in walkdir::WalkDir::new(vault_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = entry?;
        let capsula_dir = entry.path().join("_capsula");
        if !capsula_dir.exists() {
            continue;
        }

        let metadata_path = capsula_dir.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }

        let metadata_content = std::fs::read_to_string(&metadata_path)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

        // Check if ID matches
        if let Some(id) = metadata.get("id").and_then(|v| v.as_str())
            && id == run_id
        {
            return Ok(entry.path().to_path_buf());
        }

        // Check if name matches
        if let Some(name) = metadata.get("name").and_then(|v| v.as_str())
            && name == run_id
        {
            return Ok(entry.path().to_path_buf());
        }
    }

    anyhow::bail!(
        "Run with ID or name '{}' not found in vault {}",
        run_id,
        vault_dir.display()
    )
}

fn find_run_dir_by_name(vault_dir: &std::path::Path, run_name: &str) -> Result<PathBuf> {
    let mut best_match: Option<(PathBuf, Option<DateTime<chrono::FixedOffset>>)> = None;
    let mut match_count = 0usize;

    for entry in walkdir::WalkDir::new(vault_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = entry?;
        let capsula_dir = entry.path().join("_capsula");
        if !capsula_dir.exists() {
            continue;
        }

        let metadata_path = capsula_dir.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }

        let metadata_content = std::fs::read_to_string(&metadata_path)?;
        let metadata: RunMetadata = serde_json::from_str(&metadata_content)?;

        if metadata.name != run_name {
            continue;
        }

        match_count += 1;
        let timestamp = DateTime::parse_from_rfc3339(&metadata.timestamp).ok();
        match best_match {
            None => {
                best_match = Some((entry.path().to_path_buf(), timestamp));
            }
            Some((_, ref best_timestamp)) => {
                let is_newer = match (timestamp, best_timestamp) {
                    (Some(current), Some(best)) => current > *best,
                    (Some(_), None) => true,
                    _ => false,
                };
                if is_newer {
                    best_match = Some((entry.path().to_path_buf(), timestamp));
                }
            }
        }
    }

    if let Some((path, _)) = best_match {
        if match_count > 1 {
            warn!(
                "Found {} runs named '{}'; returning the newest by timestamp.",
                match_count, run_name
            );
        }
        return Ok(path);
    }

    anyhow::bail!(
        "Run with name '{}' not found in vault {}",
        run_name,
        vault_dir.display()
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "TODO: Refactor into smaller functions"
)]
#[expect(
    clippy::cognitive_complexity,
    reason = "TODO: Refactor into smaller functions"
)]
fn run() -> Result<()> {
    // Create the registry with all available hook types
    debug!("Creating hook registries");
    let pre_run_hook_registry = create_pre_run_hook_registry();
    let post_run_hook_registry = create_post_run_hook_registry();

    let cli = Cli::parse();
    let config_file_path = cli.config.unwrap_or_else(|| PathBuf::from("capsula.toml"));
    debug!("Using configuration file: {}", config_file_path.display());

    // Check if the config file exists
    if !config_file_path.exists() {
        anyhow::bail!(
            "Configuration file not found at '{}'

To get started:
  1. Create a 'capsula.toml' file in your project root
  2. Or specify a custom path with --config <path>

Example minimal configuration:
[vault]
name = \"my-project\"

[[pre-run.hooks]]
id = \"capture-git-repo\"
name = \"my-project\"
path = \".\"
",
            config_file_path.display()
        );
    }

    // Canonicalize the config file path first to get an absolute path
    let config_file_path = config_file_path.canonicalize().with_context(|| {
        format!(
            "Failed to resolve configuration file path: {}",
            config_file_path.display()
        )
    })?;

    let project_root = config_file_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine project root from config file"))?
        .to_path_buf();

    debug!("Loading configuration from: {}", config_file_path.display());
    let config = CapsulaConfig::from_file(&config_file_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_file_path.display()
        )
    })?;
    debug!("Configuration loaded successfully");

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

    // Resolve vault path with priority: CLI > env var (after dotenv) > config
    let vault_dir = resolve_vault_path(cli.vault_path, &config.vault.path, &project_root);

    match cli.command {
        Commands::List => {
            let runs = list_runs(&vault_dir)?;

            if runs.is_empty() {
                info!("No runs found in vault: {}", vault_dir.display());
                return Ok(());
            }

            let command_width = 70;

            // Print header
            println!("{:<19}  {:<20}  COMMAND", "TIMESTAMP (UTC)", "NAME");
            println!("{}", "-".repeat(19 + 2 + 20 + 2 + command_width));

            for run in runs {
                // Parse timestamp for display
                let timestamp_display = DateTime::parse_from_rfc3339(&run.timestamp).map_or_else(
                    |_| run.timestamp.clone(),
                    |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                );

                // Format command for display (truncate if too long)
                let command_display = shlex::try_join(run.command.iter().map(String::as_str))
                    .unwrap_or_else(|_| run.command.join(" "));
                let command_truncated = if command_display.len() > command_width {
                    format!("{}...", &command_display[..command_width - 3])
                } else {
                    command_display
                };

                println!(
                    "{:<19}  {:<20}  {}",
                    timestamp_display, run.name, command_truncated
                );
            }
        }
        Commands::RunDir { run_name } => {
            let run_dir = find_run_dir_by_name(&vault_dir, &run_name)?;
            println!("{}", run_dir.display());
        }
        Commands::Run { cmd } => {
            // Sanity check
            if cmd.is_empty() {
                anyhow::bail!("No command specified to run");
            }

            debug!("Creating run metadata");
            // Setup
            let run = Run::<()> {
                id: Ulid::new(),
                name: Generator::default()
                    .next()
                    .with_context(|| "Failed to generate a random name for the run")?,
                command: cmd,
                run_dir: (),
                project_root: project_root.clone(),
            };

            // Display run ID and name
            info!("Run ID: {}, Name: {}", run.id, run.name);
            debug!("Setting up run directory in vault: {}", vault_dir.display());
            let run = run.setup_run_dir(&vault_dir, 5)?;
            info!("Run directory: {}", run.run_dir.to_string_lossy());

            // Make `_capsula` directory inside run_dir to store metadata and hook outputs
            let capsula_dir = run.run_dir.join("_capsula");
            std::fs::create_dir(&capsula_dir).with_context(|| {
                format!(
                    "Failed to create _capsula directory in run directory {}",
                    run.run_dir.display()
                )
            })?;

            // Save run metadata to capsula_dir/metadata.json
            let run_metadata_path = capsula_dir.join("metadata.json");
            std::fs::write(&run_metadata_path, serde_json::to_string_pretty(&run)?).with_context(
                || {
                    format!(
                        "Failed to write metadata to {}",
                        run_metadata_path.display()
                    )
                },
            )?;

            // Pre-run hooks capture
            debug!("Executing pre-run hooks");
            let pre_params = RuntimeParams::<PreRun>::default();
            let (pre_json, should_abort) = build_and_run_hooks(
                &run,
                &pre_params,
                &config.pre_run,
                &pre_run_hook_registry,
                &project_root,
            )
            .context("Failed to execute pre-run hooks")?;
            debug!("Pre-run hooks completed");

            // Save pre_json to capsula_dir/pre.json
            let pre_json_path = capsula_dir.join("pre-run.json");
            std::fs::write(&pre_json_path, serde_json::to_string_pretty(&pre_json)?).with_context(
                || {
                    format!(
                        "Failed to write pre-run hook results to {}",
                        pre_json_path.display()
                    )
                },
            )?;

            if should_abort {
                error!("Aborting run due to pre-run hook request.");
                return Ok(());
            }

            // Execute the command
            debug!("Executing command: {:?}", run.command);
            let run_output = run.exec().context("Failed to execute command")?;
            debug!(
                "Command completed with exit code: {:?}",
                run_output.exit_code
            );
            // Save run_output to capsula_dir/command.json
            let run_json_path = capsula_dir.join("command.json");
            std::fs::write(&run_json_path, serde_json::to_string_pretty(&run_output)?)
                .with_context(|| {
                    format!("Failed to write run output to {}", run_json_path.display())
                })?;

            // Post-run hooks capture
            debug!("Executing post-run hooks");
            let post_params = RuntimeParams::<PostRun>::default();
            let (post_json, _should_abort) = build_and_run_hooks::<PostRun>(
                &run,
                &post_params,
                &config.post_run,
                &post_run_hook_registry,
                &project_root,
            )
            .context("Failed to execute post-run hooks")?;
            debug!("Post-run hooks completed");

            // Save post_json to run_dir/post.json
            let post_json_path = capsula_dir.join("post-run.json");
            std::fs::write(&post_json_path, serde_json::to_string_pretty(&post_json)?)
                .with_context(|| {
                    format!(
                        "Failed to write post-run hook results to {}",
                        post_json_path.display()
                    )
                })?;
        }
        Commands::Push {
            run_id,
            all,
            server,
        } => {
            if !all && run_id.is_none() {
                anyhow::bail!("Either provide a run ID/name or use --all flag");
            }

            // Priority: CLI flag > Environment variable (after dotenv) > Config file
            let server_url =
                resolve_server_url(server, config.server.as_deref()).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Server URL not specified. Use --server <URL>, set CAPSULA_SERVER_URL environment variable, or add 'server = \"URL\"' to capsula.toml"
                    )
                })?;

            // Create client and check if vault exists
            let client = capsula_client::CapsulaClient::new(&server_url);
            let vault_name = &config.vault.name;

            // Check if vault exists on server (only once)
            match client.vault_exists(vault_name) {
                Ok(None) => {
                    warn!(
                        "Vault '{}' does not exist on server {}. A new vault will be created.",
                        vault_name, server_url
                    );
                }
                Ok(Some(_)) => {
                    debug!("Vault '{}' exists on server", vault_name);
                }
                Err(e) => {
                    warn!("Failed to check vault existence: {}. Continuing anyway.", e);
                }
            }

            if all {
                // Push all runs in the vault
                info!(
                    "Pushing all runs from vault '{}' to server {}",
                    vault_name, server_url
                );

                let mut success_count = 0;
                let mut skip_count = 0;
                let mut error_count = 0;

                // Iterate through all run directories
                for entry in walkdir::WalkDir::new(&vault_dir)
                    .min_depth(2)
                    .max_depth(2)
                    .into_iter()
                    .filter_entry(|e| e.file_type().is_dir())
                {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            error!("Failed to read directory: {}", e);
                            error_count += 1;
                            continue;
                        }
                    };

                    let run_dir = entry.path();
                    let capsula_dir = run_dir.join("_capsula");

                    if !capsula_dir.exists() {
                        continue;
                    }

                    match push_single_run(run_dir, vault_name, &server_url, &client) {
                        Ok(()) => success_count += 1,
                        Err(e) => {
                            if e.to_string().contains("already exists") {
                                skip_count += 1;
                            } else {
                                error!("Failed to push run: {}", e);
                                error_count += 1;
                            }
                        }
                    }
                }

                info!(
                    "Push all completed: {} succeeded, {} skipped (already exist), {} failed",
                    success_count, skip_count, error_count
                );

                if error_count > 0 {
                    anyhow::bail!("{error_count} runs failed to push");
                }
            } else {
                // Push single run
                let run_id = run_id
                    .as_ref()
                    .expect("run_id must be Some when all is false");
                info!("Pushing run {} to server {}", run_id, server_url);

                let run_dir = find_run_dir(&vault_dir, run_id)?;
                push_single_run(&run_dir, vault_name, &server_url, &client)?;

                info!("Push completed successfully");
            }
        }
        Commands::Vaults { command } => match command {
            VaultsCommands::List { server } => {
                // Priority: CLI flag > Environment variable (after dotenv) > Config file
                let server_url =
                    resolve_server_url(server, config.server.as_deref()).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Server URL not specified. Use --server <URL>, set CAPSULA_SERVER_URL environment variable, or add 'server = \"URL\"' to capsula.toml"
                        )
                    })?;

                info!("Fetching vaults from server {}", server_url);

                let client = capsula_client::CapsulaClient::new(&server_url);
                let vaults = client.list_vaults().context("Failed to list vaults")?;

                if vaults.is_empty() {
                    info!("No vaults found on server");
                    return Ok(());
                }

                // Print header
                println!("{:<30}  RUNS", "VAULT NAME");
                println!("{}", "-".repeat(30 + 2 + 10));

                for vault in vaults {
                    println!("{:<30}  {}", vault.name, vault.run_count);
                }
            }
        },
    }
    Ok(())
}

fn main() {
    fmt()
        // Formatting
        .with_target(false)
        .without_time()
        .with_level(true)
        .compact()
        // Output
        .with_writer(std::io::stderr)
        // Filtering
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    if let Err(err) = run() {
        // Check for verbose mode via environment variable
        let verbose =
            std::env::var("RUST_BACKTRACE").is_ok() || std::env::var("CAPSULA_VERBOSE").is_ok();

        if verbose {
            // Show full error chain with backtrace
            error!("{err:?}");
        } else {
            // Show user-friendly error message
            error!("{err:#}\n\nFor more details, run with RUST_BACKTRACE=1");
        }

        std::process::exit(1);
    }
}
