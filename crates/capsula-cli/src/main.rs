use std::path::PathBuf;

use anyhow::{Context, Result};
use capsula_config::{CapsulaConfig, HookPhaseConfig};
use capsula_core::hook::{HookPhase, RuntimeParams};
use capsula_core::run::Run;
use chrono::DateTime;
use clap::{Parser, Subcommand};
use names::Generator;
use serde::Deserialize;
use serde_json::json;
use ulid::Ulid;

#[derive(Parser, Debug)]
#[command(name = "capsula", bin_name = "capsula", version, about = "Capsula CLI")]
struct Cli {
    #[arg(short, long, global(true))]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
    List,
}

fn create_registry() -> capsula_registry::HookRegistry {
    // Use the standard registry with all built-in hook types
    capsula_registry::standard_registry()
}

fn build_and_run_hooks(
    runtime_params: &RuntimeParams,
    hook_phase_config: &HookPhaseConfig,
    hook_registry: &capsula_registry::HookRegistry,
    project_root: &std::path::Path,
) -> Result<(Vec<serde_json::Value>, bool)> {
    let hooks = capsula_config::build_hooks(hook_phase_config, project_root, hook_registry)
        .context("Failed to build hooks from configuration")?;

    let results: Vec<_> = hooks
        .iter()
        .enumerate()
        .map(|(idx, hook)| {
            let hook_identifier = hook_phase_config
                .hooks
                .get(idx)
                .map(|config_hook| config_hook.id.clone())
                .unwrap_or_else(|| format!("hook[{}]", idx));

            let hook_config_json = hook
                .config_as_json()
                .unwrap_or_else(|_| json!({ "__error": "Failed to serialize hook config" }));

            match hook.run(runtime_params) {
                Ok(captured) => {
                    let should_abort = captured.abort_requested();

                    // Convert to JSON and add metadata object
                    let mut json = captured.to_json();
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
                    eprintln!(
                        "Warning: Failed to capture {} (config index {}): {:#}",
                        hook_identifier, idx, error
                    );
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
                            eprintln!(
                                "Warning: Failed to parse metadata from {}: {}",
                                metadata_path.display(),
                                e
                            );
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to read metadata from {}: {}",
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

fn run() -> Result<()> {
    // Create the registry with all available hook types
    let registry = create_registry();

    let cli = Cli::parse();
    let config_file_path = cli.config.unwrap_or_else(|| PathBuf::from("capsula.toml"));

    // Check if the config file exists
    if !config_file_path.exists() {
        anyhow::bail!(
            "Configuration file not found at '{}'

To get started:
  1. Create a 'capsula.toml' file in your project root
  2. Or specify a custom path with --config <path>

Example minimal configuration:
[vault]
name = \"capsula\"

[[phase.pre.hooks]]
type = \"git\"
path = \".\"",
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

    let config = CapsulaConfig::from_file(&config_file_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_file_path.display()
        )
    })?;

    // TODO: Resolving paths against project_root should be done in config parsing
    let vault_dir = if config.vault.path.is_absolute() {
        config.vault.path.clone()
    } else {
        project_root.join(&config.vault.path)
    };

    match cli.command {
        Commands::List => {
            let runs = list_runs(&vault_dir)?;

            if runs.is_empty() {
                println!("No runs found in vault: {}", vault_dir.display());
                return Ok(());
            }

            let command_width = 70;

            // Print header
            println!("{:<19}  {:<20}  COMMAND", "TIMESTAMP (UTC)", "NAME");
            println!("{}", "-".repeat(19 + 2 + 20 + 2 + command_width));

            for run in runs {
                // Parse timestamp for display
                let timestamp_display = DateTime::parse_from_rfc3339(&run.timestamp)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|_| run.timestamp.clone());

                // Format command for display (truncate if too long)
                let command_display = shlex::try_join(run.command.iter().map(|s| s.as_str()))
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
        Commands::Run { cmd } => {
            // Sanity check
            if cmd.is_empty() {
                anyhow::bail!("No command specified to run");
            }

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
            eprintln!("Run ID: {}, Name: {}", run.id, run.name);
            let run = run.setup_run_dir(&vault_dir, 5)?;
            eprintln!("Run directory: {}", run.run_dir.to_string_lossy());

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
            let pre_params = RuntimeParams {
                phase: HookPhase::Pre,
                run_dir: Some(run.run_dir.clone()),
                project_root: project_root.clone(),
            };
            let (pre_json, should_abort) =
                build_and_run_hooks(&pre_params, &config.pre_run, &registry, &project_root)
                    .context("Failed to execute pre-phase hooks")?;

            // Save pre_json to capsula_dir/pre.json
            let pre_json_path = capsula_dir.join("pre-run.json");
            std::fs::write(&pre_json_path, serde_json::to_string_pretty(&pre_json)?).with_context(
                || {
                    format!(
                        "Failed to write pre-phase results to {}",
                        pre_json_path.display()
                    )
                },
            )?;

            if should_abort {
                eprintln!("Aborting run due to pre-run hook request.");
                return Ok(());
            }

            // Execute the command
            let run_output = run.exec().context("Failed to execute command")?;
            // Save run_output to capsula_dir/command.json
            let run_json_path = capsula_dir.join("command.json");
            std::fs::write(&run_json_path, serde_json::to_string_pretty(&run_output)?)
                .with_context(|| {
                    format!("Failed to write run output to {}", run_json_path.display())
                })?;

            // Post-run hooks capture
            let post_params = RuntimeParams {
                phase: HookPhase::Post,
                run_dir: Some(run.run_dir.clone()),
                project_root: project_root.clone(),
            };
            let (post_json, _should_abort) =
                build_and_run_hooks(&post_params, &config.post_run, &registry, &project_root)
                    .context("Failed to execute post-run hooks")?;

            // Save post_json to run_dir/post.json
            let post_json_path = capsula_dir.join("post-run.json");
            std::fs::write(&post_json_path, serde_json::to_string_pretty(&post_json)?)
                .with_context(|| {
                    format!(
                        "Failed to write post-phase results to {}",
                        post_json_path.display()
                    )
                })?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        // Check for verbose mode via environment variable
        let verbose =
            std::env::var("RUST_BACKTRACE").is_ok() || std::env::var("CAPSULA_VERBOSE").is_ok();

        if verbose {
            // Show full error chain with backtrace
            eprintln!("Error: {:?}", err);
        } else {
            // Show user-friendly error message
            eprintln!("Error: {:#}", err);

            // Add hint for getting more details
            eprintln!("\nFor more details, run with RUST_BACKTRACE=1");
        }

        std::process::exit(1);
    }
}
