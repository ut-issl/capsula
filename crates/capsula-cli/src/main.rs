//! Capsula CLI main entry point
#![allow(
    clippy::print_stdout,
    reason = "Printing is acceptable in main CLI code"
)]

use anyhow::{Context, Result};
use capsula_core::hook::{PostRun, PreRun};
use capsula_core::run::PreparedRun;
use capsula_orchestration::push::push_single_run;
use capsula_orchestration::resolve::resolve_server_url;
use capsula_orchestration::run::{create_and_setup_run, run_post_hooks, run_pre_hooks};
use capsula_orchestration::setup::LoadedConfig;
use capsula_orchestration::vault::{find_run_dir, find_run_dir_by_name, list_runs};
use chrono::DateTime;
use clap::{Parser, Subcommand};
use std::io;
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

/// Exit code used when pre-run hooks capture successfully but request that the
/// command itself must not run.
const PRE_RUN_ABORT_EXIT_CODE: i32 = 125;

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

        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    /// Launch interactive terminal UI for starting and ending runs
    Tui,
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
        capsula_registry::standard_pre_run_hook_registry()?;
    let post_run_hook_registry: capsula_registry::HookRegistry<PostRun> =
        capsula_registry::standard_post_run_hook_registry()?;

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

    // TUI handles its own config loading and rendering
    if matches!(cli.command, Commands::Tui) {
        return capsula_tui::run(&config_file_path, cli.vault_path);
    }

    let LoadedConfig {
        config,
        project_root,
        vault_dir,
    } = capsula_orchestration::setup::load_config(&config_file_path, cli.vault_path)?;

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
        Commands::Show { run_name, json } => {
            let run_dir = find_run_dir_by_name(&vault_dir, &run_name)?;
            let capsula_dir = run_dir.join("_capsula");

            // Read metadata (always required)
            let metadata = capsula_orchestration::vault::read_run_metadata_json(&run_dir)?;

            // Read optional files
            let pre_run = read_json_if_exists(&capsula_dir.join("pre-run.json"))?;
            let command = read_json_if_exists(&capsula_dir.join("command.json"))?;
            let post_run = read_json_if_exists(&capsula_dir.join("post-run.json"))?;

            if json {
                let mut output = serde_json::Map::new();
                output.insert("metadata".to_string(), metadata);
                if let Some(v) = pre_run {
                    output.insert("pre_run".to_string(), v);
                }
                if let Some(v) = command {
                    output.insert("command".to_string(), v);
                }
                if let Some(v) = post_run {
                    output.insert("post_run".to_string(), v);
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::Value::Object(output))?
                );
            } else {
                // Metadata
                let id = metadata.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let name = metadata.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let timestamp = metadata
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .map_or_else(
                        || "?".to_string(),
                        |ts| {
                            DateTime::parse_from_rfc3339(ts).map_or_else(
                                |_| ts.to_string(),
                                |dt| dt.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
                            )
                        },
                    );
                let cmd_display = metadata
                    .get("command")
                    .and_then(|v| v.as_array())
                    .map(|cmd| {
                        let parts: Vec<&str> = cmd.iter().filter_map(|v| v.as_str()).collect();
                        shlex::try_join(parts.iter().copied()).unwrap_or_else(|_| parts.join(" "))
                    })
                    .unwrap_or_default();

                println!("Run:       {name}");
                println!("ID:        {id}");
                println!("Timestamp: {timestamp}");
                if !cmd_display.is_empty() {
                    println!("Command:   {cmd_display}");
                }
                if let Some(dir) = metadata.get("run_dir").and_then(|v| v.as_str()) {
                    println!("Directory: {dir}");
                }

                // Command result (single line summary)
                if let Some(ref command) = command {
                    let exit_code = command
                        .get("exit_code")
                        .and_then(serde_json::Value::as_i64)
                        .map_or_else(|| "?".to_string(), |c| c.to_string());
                    let duration_display = command.get("duration").map(|duration| {
                        let secs = duration
                            .get("secs")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0);
                        let nanos = duration
                            .get("nanos")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0);
                        format_duration(secs, nanos)
                    });
                    match duration_display {
                        Some(d) => println!("Result:    exit {exit_code} ({d})"),
                        None => println!("Result:    exit {exit_code}"),
                    }
                }
                println!();

                // Hooks summary
                if let Some(ref pre_run) = pre_run {
                    println!("Pre-run hooks:");
                    print_hook_summary(pre_run);
                }
                if let Some(ref post_run) = post_run {
                    println!("Post-run hooks:");
                    print_hook_summary(post_run);
                }
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
                std::process::exit(PRE_RUN_ABORT_EXIT_CODE);
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

            std::process::exit(run_output.exit_code);
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
                error!("Aborting run-start due to pre-run hook request.");
                std::process::exit(PRE_RUN_ABORT_EXIT_CODE);
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

            let metadata = capsula_orchestration::vault::read_run_metadata(&run_dir)?;

            let run = PreparedRun {
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

            match (all, run_id) {
                (true, _) => {
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
                }
                (false, Some(run_id)) => {
                    info!("Pushing run {} to server {}", run_id, server_url);

                    let run_dir = find_run_dir(&vault_dir, &run_id)?;
                    push_single_run(&run_dir, vault_name, &server_url, &client)?;

                    info!("Push completed successfully");
                }
                (false, None) => anyhow::bail!("Either provide a run ID/name or use --all flag"),
            }
        }
        Commands::Tui => unreachable!("Handled above"),
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

fn read_json_if_exists(path: &std::path::Path) -> Result<Option<serde_json::Value>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(Some(value))
}

fn format_duration(secs: u64, nanos: u64) -> String {
    let total_ms = secs * 1000 + nanos / 1_000_000;
    if total_ms < 1000 {
        format!("{total_ms}ms")
    } else if secs < 60 {
        format!("{secs}.{:02}s", (nanos / 10_000_000) % 100)
    } else {
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{mins}m {rem_secs}s")
    }
}

fn print_hook_summary(hooks: &serde_json::Value) {
    let Some(hooks) = hooks.as_array() else {
        return;
    };
    for hook in hooks {
        let meta = hook.get("__meta");
        let id = meta
            .and_then(|m| m.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let success = meta
            .and_then(|m| m.get("success"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if success {
            println!("  [ok]     {id}");
        } else {
            let error = meta
                .and_then(|m| m.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            println!("  [FAILED] {id}: {error}");
        }
    }
}

fn main() {
    // Check if the TUI subcommand is being invoked so we can suppress tracing
    // output that would corrupt the terminal UI.
    let is_tui = std::env::args().any(|arg| arg == "tui");

    if is_tui {
        // TUI owns the terminal; discard tracing output to avoid display corruption.
        fmt()
            .with_target(false)
            .without_time()
            .with_level(true)
            .compact()
            .with_writer(io::sink)
            .with_env_filter(EnvFilter::new("off"))
            .init();
    } else {
        fmt()
            .with_target(false)
            .without_time()
            .with_level(true)
            .compact()
            .with_writer(std::io::stderr)
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .init();
    }

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
