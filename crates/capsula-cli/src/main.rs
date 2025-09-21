use std::path::PathBuf;

use capsula_config::{CapsulaConfig, ContextPhaseConfig};
use capsula_core::context::{ContextPhase, RuntimeParams};
use capsula_core::run::Run;
use clap::{Parser, Subcommand};
use names::Generator;
use std::str::FromStr;
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
        phase: ContextPhase,
    },

    Run {
        #[arg(trailing_var_arg = true)]
        cmd: Vec<String>,
    },
}

fn create_registry() -> capsula_registry::ContextRegistry {
    // Use the standard registry with all built-in context types
    capsula_registry::standard_registry()
}

fn build_and_run_contexts(
    runtime_params: &RuntimeParams,
    context_phase_config: &ContextPhaseConfig,
    context_registry: &capsula_registry::ContextRegistry,
    project_root: &std::path::Path,
) -> anyhow::Result<Vec<Box<dyn capsula_core::captured::Captured>>> {
    let contexts =
        capsula_config::build_contexts(&context_phase_config, &project_root, &context_registry)?;

    contexts
        .iter()
        .map(|ctx| {
            let out = ctx.run_erased(&runtime_params)?;
            Ok(out)
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()
}

fn main() -> anyhow::Result<()> {
    // Create the registry with all available context types
    let registry = create_registry();

    let cli = Cli::parse();
    let config_file_path = cli
        .config
        .unwrap_or_else(|| PathBuf::from_str("capsula.toml").unwrap());
    // Check if the config file exists
    // If not, return an error
    if !config_file_path.exists() {
        anyhow::bail!(
            "Config file not found: {}",
            config_file_path.to_string_lossy()
        );
    }
    // Canonicalize the config file path first to get an absolute path
    let config_file_path = config_file_path.canonicalize()?;
    // dbg!("Using config file: {}", config_file_path.to_string_lossy());
    let project_root = config_file_path
        .parent()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to get parent directory of config file: {}",
                config_file_path.to_string_lossy()
            )
        })?
        .to_path_buf();
    // dbg!(&project_root);

    let config = CapsulaConfig::from_file(&config_file_path)?;

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
                phase: phase,
                run_dir: None,
                project_root: project_root.clone(),
            };
            let context_phase_config = match phase {
                ContextPhase::Pre => &config.phase.pre,
                ContextPhase::Post => &config.phase.post,
            };
            let context_outputs = build_and_run_contexts(
                &runtime_params,
                &context_phase_config,
                &registry,
                &project_root,
            )?;
            let context_outputs_json = context_outputs
                .iter()
                .map(|out| out.to_json())
                .collect::<Vec<_>>();
            println!("{}", serde_json::to_string_pretty(&context_outputs_json)?);
        }

        Commands::Run { cmd } => {
            // Sanity check
            if cmd.is_empty() {
                anyhow::bail!("No command specified to run");
            }

            // Setup
            let run = Run::<()> {
                id: Ulid::new(),
                name: Generator::default().next().unwrap(),
                command: cmd,
                run_dir: (),
            };
            // Display run ID and name
            eprintln!("Run ID: {}, Name: {}", run.id, run.name);
            let run = run.setup_run_dir(&vault_dir)?;
            eprintln!("Run directory: {}", run.run_dir.to_string_lossy());
            // Save run metadata to run_dir/run.json
            let run_metadata_path = run.run_dir.join("metadata.json");
            std::fs::write(&run_metadata_path, serde_json::to_string_pretty(&run)?)?;

            // Pre-run contexts capture
            let pre_params = RuntimeParams {
                phase: ContextPhase::Pre,
                run_dir: Some(run.run_dir.clone()),
                project_root: project_root.clone(),
            };
            let pre_outputs =
                build_and_run_contexts(&pre_params, &config.phase.pre, &registry, &project_root)?;
            let pre_json = pre_outputs
                .iter()
                .map(|out| out.to_json())
                .collect::<Vec<_>>();
            // Save pre_json to run_dir/pre.json
            let pre_json_path = run.run_dir.join("pre.json");
            std::fs::write(&pre_json_path, serde_json::to_string_pretty(&pre_json)?)?;

            // If any of the pre-run contexts requested abort, do not execute the command
            if pre_outputs.iter().any(|out| out.abort_requested()) {
                eprintln!("Aborting run due to pre-run context request.");
                return Ok(());
            }

            // Execute the command
            if let Ok(run_output) = run.exec() {
                // Save run_output to run_dir/run.json
                let run_json_path = run.run_dir.join("run.json");
                std::fs::write(&run_json_path, serde_json::to_string_pretty(&run_output)?)?;
            }

            // Post-run contexts capture
            let post_params = RuntimeParams {
                phase: ContextPhase::Post,
                run_dir: Some(run.run_dir.clone()),
                project_root: project_root.clone(),
            };
            let post_outputs =
                build_and_run_contexts(&post_params, &config.phase.post, &registry, &project_root)?;
            let post_json = post_outputs
                .iter()
                .map(|out| out.to_json())
                .collect::<Vec<_>>();
            // Save post_json to run_dir/post.json
            let post_json_path = run.run_dir.join("post.json");
            std::fs::write(&post_json_path, serde_json::to_string_pretty(&post_json)?)?;
        }
    }
    Ok(())
}
