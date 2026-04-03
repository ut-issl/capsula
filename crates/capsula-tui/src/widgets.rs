use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Render a clickable button and return its area for mouse hit-testing.
pub fn render_button(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    focused: bool,
    enabled: bool,
) -> Rect {
    let (bg, fg) = if !enabled {
        (Color::DarkGray, Color::Gray)
    } else if focused {
        (Color::Blue, Color::White)
    } else {
        (Color::Rgb(60, 60, 60), Color::White)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused && enabled {
            Color::Cyan
        } else {
            Color::DarkGray
        }))
        .style(Style::default().bg(bg).fg(fg));

    let paragraph = Paragraph::new(Line::from(label))
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
    area
}

/// Render a checkbox and return its area for mouse hit-testing.
pub fn render_checkbox(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    checked: bool,
    focused: bool,
) -> Rect {
    let marker = if checked { "[x]" } else { "[ ]" };
    let text = format!("{marker} {label}");

    let style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let paragraph = Paragraph::new(Line::from(text)).style(style);
    frame.render_widget(paragraph, area);
    area
}
