use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::App;

pub fn render_config_view(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = app
        .config_lines
        .iter()
        .map(|text| {
            if text.starts_with("───") {
                Line::from(Span::styled(
                    text.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if text.starts_with('>') {
                Line::from(Span::styled(
                    text.clone(),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if text.is_empty() {
                Line::from("")
            } else {
                Line::from(Span::styled(text.clone(), Style::default().fg(Color::White)))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}
