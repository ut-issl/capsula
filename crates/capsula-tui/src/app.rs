use std::time::Instant;

use anyhow::Result;
use capsula_core::hook::{PostRun, PreRun};
use capsula_core::run::Run;
use capsula_orchestration::run::{create_and_setup_run, run_post_hooks, run_pre_hooks};
use capsula_orchestration::setup::LoadedConfig;
use capsula_orchestration::vault::{RunMetadata, find_run_dir_by_name};
use ratatui::layout::Rect;
use tracing::info;

/// Which interactive widget currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    StartButton,
    InstantRunCheckbox,
    EndButton,
}

/// Active run information displayed in the TUI.
pub struct ActiveRun {
    pub run_name: String,
    pub started_at: Instant,
    pub timestamp_display: String,
}

/// A deferred action that requires a redraw before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    /// Run pre-hooks (and possibly post-hooks if instant run).
    StartRun,
    /// Run post-hooks for the active run.
    EndRun,
}

/// The main TUI application state.
pub struct App {
    pub config: LoadedConfig,
    pub instant_run: bool,
    pub error: Option<String>,
    pub status_message: Option<String>,
    pub focused: FocusTarget,
    pub should_quit: bool,
    pub confirm_quit: bool,
    pub active_run: Option<ActiveRun>,
    pub pending_action: Option<PendingAction>,
    pub last_completed_run: Option<String>,

    // Hook registries (created once)
    pre_run_registry: capsula_registry::HookRegistry<PreRun>,
    post_run_registry: capsula_registry::HookRegistry<PostRun>,

    // Clickable areas for mouse support (set during rendering)
    pub start_button_area: Option<Rect>,
    pub end_button_area: Option<Rect>,
    pub checkbox_area: Option<Rect>,
    pub confirm_yes_area: Option<Rect>,
    pub confirm_no_area: Option<Rect>,
}

impl App {
    pub fn new(config: LoadedConfig) -> Self {
        Self {
            config,
            instant_run: false,
            error: None,
            status_message: None,
            focused: FocusTarget::StartButton,
            should_quit: false,
            confirm_quit: false,
            active_run: None,
            pending_action: None,
            last_completed_run: None,
            pre_run_registry: capsula_registry::standard_pre_run_hook_registry(),
            post_run_registry: capsula_registry::standard_post_run_hook_registry(),
            start_button_area: None,
            end_button_area: None,
            checkbox_area: None,
            confirm_yes_area: None,
            confirm_no_area: None,
        }
    }

    pub const fn is_running(&self) -> bool {
        self.active_run.is_some()
    }

    /// Request starting a run. Sets a pending action so the UI can redraw
    /// with a status message before the blocking hook execution.
    pub fn request_start_run(&mut self) {
        self.error = None;
        self.last_completed_run = None;
        self.status_message = Some("Running pre-run hooks...".into());
        self.pending_action = Some(PendingAction::StartRun);
    }

    /// Request ending a run. Sets a pending action so the UI can redraw
    /// with a status message before the blocking hook execution.
    pub fn request_end_run(&mut self) {
        self.error = None;
        self.status_message = Some("Running post-run hooks...".into());
        self.pending_action = Some(PendingAction::EndRun);
    }

    /// Execute a pending action. Called by the main loop after a redraw.
    pub fn execute_pending(&mut self) {
        let Some(action) = self.pending_action.take() else {
            return;
        };

        match action {
            PendingAction::StartRun => self.do_start_run_flow(),
            PendingAction::EndRun => self.do_end_run_flow(),
        }
    }

    fn do_start_run_flow(&mut self) {
        match self.execute_start_run() {
            Ok(should_abort) => {
                if should_abort {
                    self.error =
                        Some("A pre-run hook requested abort. Run was not started.".into());
                    self.status_message = None;
                    self.active_run = None;
                    self.focused = FocusTarget::StartButton;
                    return;
                }

                if self.instant_run {
                    // Transition to ending phase
                    self.status_message = Some("Running post-run hooks...".into());
                    self.do_end_run_flow();
                } else {
                    self.status_message = None;
                    self.focused = FocusTarget::EndButton;
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to start run: {e}"));
                self.status_message = None;
                self.active_run = None;
                self.focused = FocusTarget::StartButton;
            }
        }
    }

    fn execute_start_run(&mut self) -> Result<bool> {
        let (run, capsula_dir) =
            create_and_setup_run(vec![], &self.config.project_root, &self.config.vault_dir)?;

        info!("Run created: {} (ID: {})", run.name, run.id);

        let should_abort = run_pre_hooks(
            &run,
            &capsula_dir,
            &self.config.config.pre_run,
            &self.pre_run_registry,
            &self.config.project_root,
        )?;

        let timestamp = run.timestamp();
        self.active_run = Some(ActiveRun {
            run_name: run.name,
            started_at: Instant::now(),
            timestamp_display: timestamp.format("%H:%M:%S").to_string(),
        });

        Ok(should_abort)
    }

    fn do_end_run_flow(&mut self) {
        let run_name = self
            .active_run
            .as_ref()
            .map(|a| a.run_name.clone())
            .unwrap_or_default();

        if let Err(e) = self.execute_end_run() {
            self.error = Some(format!("Failed to end run: {e}"));
        } else {
            self.last_completed_run = Some(run_name);
        }

        self.active_run = None;
        self.status_message = None;
        self.focused = FocusTarget::StartButton;
    }

    fn execute_end_run(&self) -> Result<()> {
        let active = self
            .active_run
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active run"))?;

        let run_dir = find_run_dir_by_name(&self.config.vault_dir, &active.run_name)?;
        let capsula_dir = run_dir.join("_capsula");

        let metadata_path = capsula_dir.join("metadata.json");
        let metadata_content = std::fs::read_to_string(&metadata_path)?;
        let metadata: RunMetadata = serde_json::from_str(&metadata_content)?;

        let run = Run {
            id: metadata.id,
            name: metadata.name,
            command: metadata.command,
            run_dir,
            project_root: self.config.project_root.clone(),
        };

        info!("Finalizing run: {} (ID: {})", run.name, run.id);

        run_post_hooks(
            &run,
            &capsula_dir,
            &self.config.config.post_run,
            &self.post_run_registry,
            &self.config.project_root,
        )?;

        info!("Run '{}' finalized successfully", run.name);
        Ok(())
    }

    pub const fn toggle_instant_run(&mut self) {
        self.instant_run = !self.instant_run;
    }

    pub const fn request_quit(&mut self) {
        if self.is_running() {
            self.confirm_quit = true;
        } else {
            self.should_quit = true;
        }
    }

    pub const fn confirm_quit(&mut self) {
        self.should_quit = true;
    }

    pub const fn cancel_quit(&mut self) {
        self.confirm_quit = false;
    }

    pub const fn cycle_focus_forward(&mut self) {
        self.focused = if self.is_running() {
            FocusTarget::EndButton
        } else {
            match self.focused {
                FocusTarget::StartButton => FocusTarget::InstantRunCheckbox,
                FocusTarget::InstantRunCheckbox | FocusTarget::EndButton => {
                    FocusTarget::StartButton
                }
            }
        };
    }

    pub fn activate_focused(&mut self) {
        match self.focused {
            FocusTarget::StartButton if !self.is_running() => self.request_start_run(),
            FocusTarget::InstantRunCheckbox if !self.is_running() => self.toggle_instant_run(),
            FocusTarget::EndButton if self.is_running() => self.request_end_run(),
            _ => {}
        }
    }
}
