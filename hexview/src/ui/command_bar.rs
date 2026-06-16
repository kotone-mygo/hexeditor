use ratatui::{Frame, layout::Rect, widgets::Paragraph, style::Style};
use crate::app::App;

pub fn render_command_bar(frame: &mut Frame, area: Rect, app: &App) {
    let (text, style) = match app.mode {
        crate::app::Mode::Command => (
            format!(":{}", app.command_line),
            Style::default(),
        ),
        crate::app::Mode::Search => {
            let prompt = if app.search_reverse { "?" } else { "/" };
            let t = if !app.search_results.is_empty() {
                format!(
                    "{}{}  ({}/{})",
                    prompt,
                    app.command_line,
                    app.search_index + 1,
                    app.search_results.len()
                )
            } else {
                format!("{}{}", prompt, app.command_line)
            };
            (t, Style::default())
        }
        _ => {
            if !app.search_results.is_empty() {
                (
                    format!(
                        "({}/{})",
                        app.search_index + 1,
                        app.search_results.len()
                    ),
                    Style::default().fg(ratatui::style::Color::DarkGray),
                )
            } else {
                (String::new(), Style::default())
            }
        }
    };

    frame.render_widget(Paragraph::new(text).style(style), area);
}
