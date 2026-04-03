use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, FocusTarget};
use crate::widgets::{render_button, render_checkbox};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Outer frame
    let outer_block = Block::default()
        .title(" capsula ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(outer_block, area);

    let inner = area.inner(Margin::new(2, 1));

    if app.is_running() {
        draw_active_state(frame, inner, app);
    } else {
        draw_idle_state(frame, inner, app);
    }

    // Status message (shown during hook execution)
    if let Some(ref status) = app.status_message {
        draw_status(frame, area, status);
    }

    // Success message after completed run
    if let Some(ref run_name) = app.last_completed_run {
        draw_success(frame, area, run_name);
    }

    // Error display at bottom
    if let Some(ref error) = app.error {
        draw_error(frame, area, error);
    }

    // Footer with keybindings
    draw_footer(frame, area);

    // Confirm quit overlay
    if app.confirm_quit {
        draw_confirm_quit(frame, area, app);
    }
}

fn draw_idle_state(frame: &mut Frame, area: Rect, app: &mut App) {
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
    app.checkbox_area = Some(render_checkbox(
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
    app.start_button_area = Some(render_button(
        frame,
        button_area,
        "Start Run",
        app.focused == FocusTarget::StartButton,
        true,
    ));
}

fn draw_active_state(frame: &mut Frame, area: Rect, app: &mut App) {
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
    app.end_button_area = Some(render_button(
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

fn draw_success(frame: &mut Frame, area: Rect, run_name: &str) {
    let success_y = area.height.saturating_sub(4);
    let success_area = Rect::new(area.x + 1, success_y, area.width.saturating_sub(2), 1);

    let success_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            "✓ ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Run '{run_name}' completed successfully"),
            Style::default().fg(Color::Green),
        ),
    ]));
    frame.render_widget(success_widget, success_area);
}

fn draw_error(frame: &mut Frame, area: Rect, error: &str) {
    // Position error near the bottom, above the footer
    let error_y = area.height.saturating_sub(4);
    let error_area = Rect::new(area.x + 1, error_y, area.width.saturating_sub(2), 2);

    let error_widget = Paragraph::new(Line::from(vec![
        Span::styled(
            "Error: ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(error, Style::default().fg(Color::Red)),
    ]))
    .wrap(Wrap { trim: true });

    frame.render_widget(error_widget, error_area);
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

fn draw_confirm_quit(frame: &mut Frame, area: Rect, app: &mut App) {
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

    app.confirm_yes_area = Some(yes_area);
    app.confirm_no_area = Some(no_area);
}
