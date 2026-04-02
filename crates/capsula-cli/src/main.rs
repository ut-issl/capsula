//! Capsula CLI main entry point
#![allow(
    clippy::print_stdout,
    reason = "Printing is acceptable in main CLI code"
)]

use anyhow::{Context, Result};
use capsula_config::CapsulaConfig;
use capsula_core::hook::{PostRun, PreRun};
use capsula_core::run::Run;
use capsula_orchestration::push::push_single_run;
use capsula_orchestration::resolve::{resolve_server_url, resolve_vault_path};
use capsula_orchestration::run::{create_and_setup_run, run_post_hooks, run_pre_hooks};
use capsula_orchestration::vault::{find_run_dir, find_run_dir_by_name, list_runs};
use chrono::DateTime;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

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
    /// Start a manual run: create run directory and execute pre-run hooks
    ///
    /// Use this when you want to capture context before and after an external
    /// process (e.g., a GUI-triggered analysis) that capsula does not manage.
    /// The auto-generated run name is printed to stdout so callers can capture it
    /// (e.g., `name=$(capsula run-start)`), then later finalize with `run-end`.
    RunStart,
    /// End a manual run: execute post-run hooks for an existing run
    ///
    /// Finalizes a run previously started with `run-start`.
    RunEnd {
        /// Name of the run to finalize (as printed by `run-start`)
        run_name: String,
    },
    /// Print the run directory for a given run name
    RunDir {
        /// Run name to locate (e.g., happy-river)
        run_name: String,
    },
    List,
    /// Show detailed information about a specific run
    Show {
        /// Run name to display (e.g., happy-river)
        run_name: String,
    },
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

#[expect(
    clippy::too_many_lines,
    reason = "CLI dispatch function; further splitting would hurt readability"
)]
fn run() -> Result<()> {
    debug!("Creating hook registries");
    let pre_run_hook_registry: capsula_registry::HookRegistry<PreRun> =
        capsula_registry::standard_pre_run_hook_registry();
    let post_run_hook_registry: capsula_registry::HookRegistry<PostRun> =
        capsula_registry::standard_post_run_hook_registry();

    let cli = Cli::parse();
    let config_file_path = cli.config.unwrap_or_else(|| PathBuf::from("capsula.toml"));
    debug!("Using configuration file: {}", config_file_path.display());

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
                let timestamp_display = DateTime::parse_from_rfc3339(&run.timestamp).map_or_else(
                    |_| run.timestamp.clone(),
                    |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                );

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
        Commands::Show { run_name } => {
            let run_dir = find_run_dir_by_name(&vault_dir, &run_name)?;
            let capsula_dir = run_dir.join("_capsula");

            // Read and display metadata
            let metadata_path = capsula_dir.join("metadata.json");
            let metadata: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&metadata_path)
                    .with_context(|| format!("Failed to read {}", metadata_path.display()))?,
            )?;

            println!("=== Run: {run_name} ===");
            println!();

            // Metadata
            println!("--- Metadata ---");
            if let Some(id) = metadata.get("id").and_then(|v| v.as_str()) {
                println!("  ID:        {id}");
            }
            if let Some(name) = metadata.get("name").and_then(|v| v.as_str()) {
                println!("  Name:      {name}");
            }
            if let Some(ts) = metadata.get("timestamp").and_then(|v| v.as_str()) {
                let display = DateTime::parse_from_rfc3339(ts).map_or_else(
                    |_| ts.to_string(),
                    |dt| dt.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
                );
                println!("  Timestamp: {display}");
            }
            if let Some(cmd) = metadata.get("command").and_then(|v| v.as_array()) {
                let parts: Vec<&str> = cmd.iter().filter_map(|v| v.as_str()).collect();
                let display =
                    shlex::try_join(parts.iter().copied()).unwrap_or_else(|_| parts.join(" "));
                println!("  Command:   {display}");
            }
            if let Some(dir) = metadata.get("run_dir").and_then(|v| v.as_str()) {
                println!("  Run Dir:   {dir}");
            }
            println!();

            // Pre-run hooks
            let pre_run_path = capsula_dir.join("pre-run.json");
            if pre_run_path.exists() {
                let pre_run: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&pre_run_path)?)?;
                println!("--- Pre-run Hooks ---");
                print_hook_outputs(&pre_run);
                println!();
            }

            // Command output
            let command_path = capsula_dir.join("command.json");
            if command_path.exists() {
                let command: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&command_path)?)?;
                println!("--- Command Output ---");
                if let Some(exit_code) = command.get("exit_code") {
                    println!("  Exit Code: {exit_code}");
                }
                if let Some(duration) = command.get("duration") {
                    let secs = duration
                        .get("secs")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let nanos = duration
                        .get("nanos")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let total_ms = secs * 1000 + nanos / 1_000_000;
                    println!("  Duration:  {total_ms}ms");
                }
                if let Some(stdout) = command.get("stdout").and_then(|v| v.as_str())
                    && !stdout.is_empty()
                {
                    println!("  Stdout:");
                    for line in stdout.lines() {
                        println!("    {line}");
                    }
                }
                if let Some(stderr) = command.get("stderr").and_then(|v| v.as_str())
                    && !stderr.is_empty()
                {
                    println!("  Stderr:");
                    for line in stderr.lines() {
                        println!("    {line}");
                    }
                }
                println!();
            }

            // Post-run hooks
            let post_run_path = capsula_dir.join("post-run.json");
            if post_run_path.exists() {
                let post_run: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&post_run_path)?)?;
                println!("--- Post-run Hooks ---");
                print_hook_outputs(&post_run);
                println!();
            }
        }
        Commands::RunDir { run_name } => {
            let run_dir = find_run_dir_by_name(&vault_dir, &run_name)?;
            println!("{}", run_dir.display());
        }
        Commands::Run { cmd } => {
            if cmd.is_empty() {
                anyhow::bail!("No command specified to run");
            }

            let (run, capsula_dir) = create_and_setup_run(cmd, &project_root, &vault_dir)?;

            let should_abort = run_pre_hooks(
                &run,
                &capsula_dir,
                &config.pre_run,
                &pre_run_hook_registry,
                &project_root,
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
            let run_json_path = capsula_dir.join("command.json");
            std::fs::write(&run_json_path, serde_json::to_string_pretty(&run_output)?)
                .with_context(|| {
                    format!("Failed to write run output to {}", run_json_path.display())
                })?;

            run_post_hooks(
                &run,
                &capsula_dir,
                &config.post_run,
                &post_run_hook_registry,
                &project_root,
            )?;
        }
        Commands::RunStart => {
            let (run, capsula_dir) = create_and_setup_run(vec![], &project_root, &vault_dir)?;

            let should_abort = run_pre_hooks(
                &run,
                &capsula_dir,
                &config.pre_run,
                &pre_run_hook_registry,
                &project_root,
            )?;
            if should_abort {
                warn!("A pre-run hook requested abort.");
            }

            // Print run name to stdout for callers to capture
            println!("{}", run.name);
        }
        Commands::RunEnd { run_name } => {
            let run_dir = find_run_dir_by_name(&vault_dir, &run_name)?;
            let capsula_dir = run_dir.join("_capsula");

            let post_run_path = capsula_dir.join("post-run.json");
            if post_run_path.exists() {
                anyhow::bail!(
                    "Run '{run_name}' has already been finalized (post-run.json already exists)"
                );
            }

            let metadata_path = capsula_dir.join("metadata.json");
            let metadata_content = std::fs::read_to_string(&metadata_path).with_context(|| {
                format!("Failed to read metadata from {}", metadata_path.display())
            })?;
            let metadata: capsula_orchestration::vault::RunMetadata =
                serde_json::from_str(&metadata_content).with_context(|| {
                    format!("Failed to parse metadata from {}", metadata_path.display())
                })?;

            let run = Run {
                id: metadata.id,
                name: metadata.name,
                command: metadata.command,
                run_dir,
                project_root: project_root.clone(),
            };

            info!("Finalizing run: {} (ID: {})", run.name, run.id);

            run_post_hooks(
                &run,
                &capsula_dir,
                &config.post_run,
                &post_run_hook_registry,
                &project_root,
            )?;

            info!("Run '{}' finalized successfully", run_name);
        }
        Commands::Push {
            run_id,
            all,
            server,
        } => {
            if !all && run_id.is_none() {
                anyhow::bail!("Either provide a run ID/name or use --all flag");
            }

            let server_url =
                resolve_server_url(server, config.server.as_deref()).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Server URL not specified. Use --server <URL>, set CAPSULA_SERVER_URL environment variable, or add 'server = \"URL\"' to capsula.toml"
                    )
                })?;

            let client = capsula_client::CapsulaClient::new(&server_url);
            let vault_name = &config.vault.name;

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
                info!(
                    "Pushing all runs from vault '{}' to server {}",
                    vault_name, server_url
                );

                let mut success_count = 0;
                let mut skip_count = 0;
                let mut error_count = 0;

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

fn print_hook_outputs(hooks: &serde_json::Value) {
    let Some(hooks) = hooks.as_array() else {
        return;
    };
    for hook in hooks {
        let id = hook
            .get("__meta")
            .and_then(|m| m.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let success = hook
            .get("__meta")
            .and_then(|m| m.get("success"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let status = if success { "ok" } else { "FAILED" };
        println!("  [{status}] {id}");

        if !success
            && let Some(error) = hook
                .get("__meta")
                .and_then(|m| m.get("error"))
                .and_then(|v| v.as_str())
        {
            println!("    Error: {error}");
        }

        // Print hook-specific fields (skip __meta)
        if let Some(obj) = hook.as_object() {
            for (key, value) in obj {
                if key == "__meta" {
                    continue;
                }
                let display = serde_json::to_string(value).unwrap_or_default();
                // Truncate long values
                if display.len() > 120 {
                    println!("    {key}: {}...", &display[..117]);
                } else {
                    println!("    {key}: {display}");
                }
            }
        }
    }
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
