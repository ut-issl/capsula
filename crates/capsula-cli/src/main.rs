use std::path::PathBuf;

use anyhow::{Context, Result};
use capsula_config::{CapsulaConfig, HookPhaseConfig};
use capsula_core::hook::{HookPhase, RuntimeParams};
use capsula_core::run::Run;
use clap::{Parser, Subcommand};
use names::Generator;
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
    Capture {
        #[arg(short, long, default_value = "pre")]
        phase: HookPhase,
    },

    Run {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
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
        .map(|(idx, ctx)| {
            let hook_identifier = hook_phase_config
                .hooks
                .get(idx)
                .map(|config_ctx| config_ctx.id.clone())
                .unwrap_or_else(|| format!("hook[{}]", idx));

            match ctx.run_erased(runtime_params) {
                Ok(captured) => {
                    let should_abort = captured.abort_requested();

                    // Convert to JSON and add metadata object
                    let mut json = captured.to_json();
                    if let serde_json::Value::Object(ref mut map) = json {
                        let metadata = json!({
                            "success": true,
                            "index": idx
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
                            "success": false,
                            "index": idx,
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
    // dbg!(&vault_dir);

    match cli.command {
        Commands::Capture { phase } => {
            let runtime_params = RuntimeParams {
                phase,
                run_dir: None,
                project_root: project_root.clone(),
            };
            let hook_phase_config = match phase {
                HookPhase::Pre => &config.phase.pre,
                HookPhase::Post => &config.phase.post,
            };
            let (output_json, _should_abort) =
                build_and_run_hooks(&runtime_params, hook_phase_config, &registry, &project_root)?;

            println!("{}", serde_json::to_string_pretty(&output_json)?);
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
            };
            // Display run ID and name
            eprintln!("Run ID: {}, Name: {}", run.id, run.name);
            let run = run.setup_run_dir(&vault_dir)?;
            eprintln!("Run directory: {}", run.run_dir.to_string_lossy());
            // Save run metadata to run_dir/metadata.json
            let run_metadata_path = run.run_dir.join("metadata.json");
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
                build_and_run_hooks(&pre_params, &config.phase.pre, &registry, &project_root)
                    .context("Failed to execute pre-phase hooks")?;

            // Save pre_json to run_dir/pre.json
            let pre_json_path = run.run_dir.join("pre.json");
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
            // Save run_output to run_dir/run.json
            let run_json_path = run.run_dir.join("run.json");
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
                build_and_run_hooks(&post_params, &config.phase.post, &registry, &project_root)
                    .context("Failed to execute post-run hooks")?;

            // Save post_json to run_dir/post.json
            let post_json_path = run.run_dir.join("post.json");
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
