use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::App;

pub fn render_help_view(frame: &mut Frame, area: Rect, app: &App) {
    let scroll = app.help_scroll as usize;
    let lines: Vec<Line> = app
        .help_lines
        .iter()
        .enumerate()
        .skip(scroll)
        .map(|(_, text)| {
            let is_header = text.starts_with("───");
            let style = if is_header {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if text.is_empty() {
                Style::default()
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(text.clone(), style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}
