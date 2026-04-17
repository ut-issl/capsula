mod app;
mod event;
mod ui;
mod widgets;

use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;
use event::AppEvent;

/// Launch the interactive TUI.
///
/// `config_path` is the path to `capsula.toml`.
/// `vault_path_override` is an optional CLI override for the vault path.
pub fn run(config_path: &Path, vault_path_override: Option<PathBuf>) -> Result<()> {
    let loaded = capsula_orchestration::setup::load_config(config_path, vault_path_override)?;

    // Install panic hook that restores the terminal before printing the panic message
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to set up terminal")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    let mut app = App::new(loaded);

    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal regardless of result
    disable_raw_mode().context("Failed to disable raw mode")?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to restore terminal")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        let mut hit_areas = app.hit_areas;
        terminal
            .draw(|frame| hit_areas = ui::draw(frame, app))
            .context("Failed to draw frame")?;
        app.hit_areas = hit_areas;

        // Execute pending actions after the redraw so the status message is visible
        if app.pending_action.is_some() {
            app.execute_pending();
            continue; // Redraw immediately to show the result
        }

        match event::next_event()? {
            AppEvent::Key(key) => handle_key(app, key),
            AppEvent::Mouse(mouse) => handle_mouse(app, mouse),
            AppEvent::Tick => {} // Elapsed timer updates on next draw
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    // In confirm-quit mode, only y/n/Esc are handled
    if app.confirm_quit {
        match key.code {
            KeyCode::Char('y' | 'Y') => app.confirm_quit(),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => app.cancel_quit(),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.request_quit(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.request_quit(),
        KeyCode::Tab => app.cycle_focus_forward(),
        KeyCode::Enter | KeyCode::Char(' ') => app.activate_focused(),
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
        return;
    }

    let x = mouse.column;
    let y = mouse.row;

    // In confirm-quit mode, only handle yes/no clicks
    if app.confirm_quit {
        if let Some(area) = app.hit_areas.confirm_yes
            && hit_test(x, y, area)
        {
            app.confirm_quit();
            return;
        }
        if let Some(area) = app.hit_areas.confirm_no
            && hit_test(x, y, area)
        {
            app.cancel_quit();
        }
        return;
    }

    // Check interactive widgets
    if app.is_running() {
        if let Some(area) = app.hit_areas.end_button
            && hit_test(x, y, area)
        {
            app.request_end_run();
        }
    } else {
        if let Some(area) = app.hit_areas.start_button
            && hit_test(x, y, area)
        {
            app.request_start_run();
            return;
        }
        if let Some(area) = app.hit_areas.checkbox
            && hit_test(x, y, area)
        {
            app.toggle_instant_run();
        }
    }
}

const fn hit_test(x: u16, y: u16, area: ratatui::layout::Rect) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}
