use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::App;

pub fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    if app.tabs.len() <= 1 {
        return;
    }

    let mut spans = Vec::new();
    for (i, tab) in app.tabs.iter().enumerate() {
        let style = if i == app.active_tab {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let modified = if tab.modified { " *" } else { "" };
        spans.push(Span::styled(format!(" {}{} ", tab.name, modified), style));
        spans.push(Span::raw(" "));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
