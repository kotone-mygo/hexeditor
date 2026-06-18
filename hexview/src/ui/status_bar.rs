use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use hexcore::SelectionMode;
use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mode_str = match app.mode {
        crate::app::Mode::Normal => "NORMAL",
        crate::app::Mode::Insert => "INSERT",
        crate::app::Mode::Replace => "REPLACE",
        crate::app::Mode::ReplaceOnce => "REPLACE",
        crate::app::Mode::VisualChar => "VISUAL",
        crate::app::Mode::VisualLine => "VISUAL LINE",
        crate::app::Mode::VisualBlock => "VISUAL BLOCK",
        crate::app::Mode::Command => "COMMAND",
        crate::app::Mode::Search => "SEARCH",
    };

    let filename = app
        .tabs
        .get(app.active_tab)
        .map(|t| t.name.as_str())
        .unwrap_or("[No File]");

    let modified = if app.buffer.is_modified() { " [+]" } else { "" };
    let offset_str = format!("0x{:08X}", app.cursor.offset);
    let selection_str = if app.cursor.selection_mode == SelectionMode::Block {
        let (top, bottom, left, right) = app.cursor.block_bounds(app.nibble_mode);
        let rows = bottom - top + 1;
        let cols = right - left + 1;
        format!(" Sel:{}", rows * cols)
    } else if let (Some(s), Some(e)) = (app.cursor.selection_start(), app.cursor.selection_end()) {
        format!(" Sel:{}", e - s + 1)
    } else {
        String::new()
    };

    let endian_preview = if app.cursor.offset + 2 <= app.buffer.len() {
        if let Ok(bytes) = app.buffer.read(app.cursor.offset, 2) {
            let u16_le = u16::from_le_bytes([bytes[0], bytes[1]]);
            let u16_be = u16::from_be_bytes([bytes[0], bytes[1]]);
            format!(" u16 LE:0x{:04X} BE:0x{:04X}", u16_le, u16_be)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let nibble_str = if app.nibble_mode { " NIBBLE" } else { "" };
    let left = format!(" {}{} {} {} {}  ", mode_str, nibble_str, filename, modified, offset_str);
    let right = format!("{}{}", selection_str, endian_preview);

    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::Cyan)),
        Span::styled(right, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}
