use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, CompletedRun, FocusTarget, HitAreas};
use crate::widgets::{render_button, render_checkbox};

pub fn draw(frame: &mut Frame, app: &App) -> HitAreas {
    let area = frame.area();
    let mut hit = HitAreas::default();

    // Outer frame
    let outer_block = Block::default()
        .title(" capsula ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(outer_block, area);

    let inner = area.inner(Margin::new(2, 1));

    if app.is_running() {
        draw_active_state(frame, inner, app, &mut hit);
    } else {
        draw_idle_state(frame, inner, app, &mut hit);
    }

    // Status message (shown during hook execution)
    if let Some(ref status) = app.status_message {
        draw_status(frame, area, status);
    }

    // Result message after completed run
    if let Some(ref completed) = app.last_completed_run {
        draw_completed(frame, area, completed);
    }

    // Error display at bottom
    if let Some(ref error) = app.error {
        draw_error(frame, area, error);
    }

    // Footer with keybindings
    draw_footer(frame, area);

    // Confirm quit overlay
    if app.confirm_quit {
        draw_confirm_quit(frame, area, &mut hit);
    }

    hit
}

fn draw_idle_state(frame: &mut Frame, area: Rect, app: &App, hit: &mut HitAreas) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Vault info
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Checkbox
        Constraint::Length(1), // Spacing
        Constraint::Length(3), // Start button
    ])
    .split(area);

    // Vault info
    draw_vault_info(frame, chunks[0], app);

    // Instant run checkbox
    let checkbox_area = Rect::new(chunks[2].x, chunks[2].y, chunks[2].width.min(30), 1);
    hit.checkbox = Some(render_checkbox(
        frame,
        checkbox_area,
        "Instant run",
        app.instant_run,
        app.focused == FocusTarget::InstantRunCheckbox,
    ));

    // Start button
    let button_width = area.width.min(30);
    let button_x = area.x + (area.width.saturating_sub(button_width)) / 2;
    let button_area = Rect::new(button_x, chunks[4].y, button_width, 3);
    hit.start_button = Some(render_button(
        frame,
        button_area,
        "Start Run",
        app.focused == FocusTarget::StartButton,
        true,
    ));
}

fn draw_active_state(frame: &mut Frame, area: Rect, app: &App, hit: &mut HitAreas) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Vault info
        Constraint::Length(1), // Spacing
        Constraint::Length(4), // Run info
        Constraint::Length(1), // Spacing
        Constraint::Length(3), // End button
    ])
    .split(area);

    // Vault info
    draw_vault_info(frame, chunks[0], app);

    // Run info
    if let Some(ref active) = app.active_run {
        let elapsed = active.started_at.elapsed();
        let total_secs = elapsed.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        let run_info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Run:     ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &active.run_name,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Started: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&active.timestamp_display),
            ]),
            Line::from(vec![
                Span::styled("Elapsed: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{hours:02}:{minutes:02}:{seconds:02}"),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        ]);
        frame.render_widget(run_info, chunks[2]);
    }

    // End button
    let button_width = area.width.min(30);
    let button_x = area.x + (area.width.saturating_sub(button_width)) / 2;
    let button_area = Rect::new(button_x, chunks[4].y, button_width, 3);
    hit.end_button = Some(render_button(
        frame,
        button_area,
        "End Run",
        app.focused == FocusTarget::EndButton,
        true,
    ));
}

fn draw_vault_info(frame: &mut Frame, area: Rect, app: &App) {
    let vault_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Vault: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &app.config.config.vault.name,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Path:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(app.config.vault_dir.to_string_lossy().to_string()),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Config "),
    );
    frame.render_widget(vault_info, area);
}

fn draw_status(frame: &mut Frame, area: Rect, status: &str) {
    // Position status near the bottom, above the footer
    let status_y = area.height.saturating_sub(4);
    let status_area = Rect::new(area.x + 1, status_y, area.width.saturating_sub(2), 1);

    let status_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            "⏳ ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(status, Style::default().fg(Color::Yellow)),
    ]));
    frame.render_widget(status_widget, status_area);
}

fn draw_completed(frame: &mut Frame, area: Rect, completed: &CompletedRun) {
    let run_name = &completed.run_name;

    let (mark, message, color) = if completed.hook_failures.is_empty() {
        (
            "✓ ",
            format!("Run '{run_name}' completed successfully"),
            Color::Green,
        )
    } else {
        (
            "! ",
            format!("Run '{run_name}' completed, but a post-run hook failed:"),
            Color::Yellow,
        )
    };

    let style = Style::default().fg(color);
    let lines = std::iter::once(Line::from(vec![
        Span::styled(mark, style.add_modifier(Modifier::BOLD)),
        Span::styled(message, style),
    ]))
    .chain(
        completed
            .hook_failures
            .iter()
            .map(|failure| Line::styled(format!("  {failure}"), style)),
    )
    .collect();
    draw_message(frame, area, lines);
}

fn draw_error(frame: &mut Frame, area: Rect, error: &str) {
    let style = Style::default().fg(Color::Red);
    let mut lines = error.lines();
    let first = Line::from(vec![
        Span::styled("Error: ", style.add_modifier(Modifier::BOLD)),
        Span::styled(lines.next().unwrap_or_default(), style),
    ]);
    let lines = std::iter::once(first)
        .chain(lines.map(|line| Line::styled(line, style)))
        .collect();
    draw_message(frame, area, lines);
}

/// Rows from the top of the screen that the idle state uses: the border, the
/// idle-state widgets (see [`draw_idle_state`]), and one blank row.
const IDLE_CONTENT_ROWS: u16 = 1 + 9 + 1;

/// Draw a message above the footer. The message grows upwards to fit its
/// lines, but not into the idle-state widgets, and wraps long lines.
fn draw_message(frame: &mut Frame, area: Rect, lines: Vec<Line<'_>>) {
    let width = area.width.saturating_sub(2);
    let needed = lines
        .iter()
        .map(|line| wrapped_height(line.width(), width))
        .sum();
    let message_area = Rect {
        x: area.x + 1,
        width,
        ..message_rows(area.height, needed)
    };
    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(widget, message_area);
}

/// Number of rows a line of `line_width` columns takes when wrapped to `width`.
fn wrapped_height(line_width: usize, width: u16) -> u16 {
    let rows = line_width.div_ceil(usize::from(width.max(1))).max(1);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// The rows of the message area (only `y` and `height` are set) for a
/// message that needs `needed` rows on a screen `screen_height` rows tall.
///
/// The area ends two rows above the footer and keeps at least two rows, as
/// before messages could grow.
fn message_rows(screen_height: u16, needed: u16) -> Rect {
    let bottom = screen_height.saturating_sub(2);
    let max_height = bottom.saturating_sub(IDLE_CONTENT_ROWS).max(2);
    let height = needed.clamp(2, max_height);
    Rect {
        y: bottom.saturating_sub(height),
        height,
        ..Rect::default()
    }
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer_y = area.height.saturating_sub(1);
    let footer_area = Rect::new(area.x + 1, footer_y, area.width.saturating_sub(2), 1);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" focus  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Enter/Space",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" activate  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "click",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" buttons", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center);

    frame.render_widget(footer, footer_area);
}

fn draw_confirm_quit(frame: &mut Frame, area: Rect, hit: &mut HitAreas) {
    let popup_width = 46_u16;
    let popup_height = 7_u16;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Confirm Quit ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::vertical([
        Constraint::Length(2), // Message
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Buttons
    ])
    .split(inner);

    let message = Paragraph::new("A run is currently active.\nAre you sure you want to quit?")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(message, chunks[0]);

    // Yes / No buttons as text
    let button_area = chunks[2];
    let yes_width = 8_u16;
    let no_width = 8_u16;
    let gap = 4_u16;
    let total = yes_width + gap + no_width;
    let start_x = button_area.x + (button_area.width.saturating_sub(total)) / 2;

    let yes_area = Rect::new(start_x, button_area.y, yes_width, 1);
    let no_area = Rect::new(start_x + yes_width + gap, button_area.y, no_width, 1);

    let yes_style = Style::default()
        .fg(Color::White)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD);
    let no_style = Style::default()
        .fg(Color::White)
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(
        Paragraph::new(" [Y]es ")
            .style(yes_style)
            .alignment(Alignment::Center),
        yes_area,
    );
    frame.render_widget(
        Paragraph::new("  [N]o  ")
            .style(no_style)
            .alignment(Alignment::Center),
        no_area,
    );

    hit.confirm_yes = Some(yes_area);
    hit.confirm_no = Some(no_area);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "Tests use unwrap for brevity")]

    use super::*;

    #[test]
    fn wrapped_height_counts_wrapped_rows() {
        assert_eq!(wrapped_height(0, 10), 1);
        assert_eq!(wrapped_height(10, 10), 1);
        assert_eq!(wrapped_height(11, 10), 2);
    }

    #[test]
    fn message_rows_grows_upwards_to_fit() {
        // A 30-row screen: the message ends at row 28 (exclusive).
        assert_eq!(message_rows(30, 1), Rect::new(0, 26, 0, 2));
        assert_eq!(message_rows(30, 5), Rect::new(0, 23, 0, 5));
    }

    #[test]
    fn message_rows_stops_at_idle_widgets() {
        let rows = message_rows(30, 100);
        assert_eq!(rows.y, IDLE_CONTENT_ROWS);
        assert_eq!(rows.height, 30 - 2 - IDLE_CONTENT_ROWS);
    }

    #[test]
    fn message_rows_keeps_two_rows_on_small_screens() {
        assert_eq!(message_rows(12, 5), Rect::new(0, 8, 0, 2));
    }

    #[test]
    fn draw_completed_shows_every_failed_hook() {
        let completed = CompletedRun {
            run_name: "quiet-lake".to_string(),
            hook_failures: (0..3)
                .map(|i| format!("post[{i}] capture-command: command exited with status 1"))
                .collect(),
        };
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();

        terminal
            .draw(|frame| draw_completed(frame, frame.area(), &completed))
            .unwrap();

        let screen = format!("{:?}", terminal.backend().buffer());
        for i in 0..3 {
            assert!(
                screen.contains(&format!("post[{i}] capture-command")),
                "missing failure {i}:\n{screen}"
            );
        }
    }
}
