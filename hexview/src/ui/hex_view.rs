use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::{App, Mode};

pub fn render_hex_view(frame: &mut Frame, area: Rect, app: &App) {
    let bytes_per_row = app.cursor.bytes_per_row as usize;
    let cursor_offset = app.cursor.offset;
    let current_row = if bytes_per_row > 0 {
        cursor_offset / bytes_per_row as u64
    } else {
        0
    };
    let half = (area.height / 2) as u64;
    let start_row = current_row.saturating_sub(half);

    let mut lines = Vec::new();

    for row in start_row..start_row + area.height as u64 {
        let row_offset = row * bytes_per_row as u64;
        if row_offset >= app.buffer.len() {
            lines.push(Line::from(""));
            continue;
        }

        let mut spans = Vec::new();

        spans.push(Span::styled(
            format!("{:08X}  ", row_offset),
            Style::default().fg(Color::DarkGray),
        ));

        for col in 0..bytes_per_row {
            let byte_off = row_offset + col as u64;
            if col > 0 && col % 8 == 0 {
                spans.push(Span::raw(" "));
            }
            if byte_off >= app.buffer.len() {
                spans.push(Span::raw("   "));
                continue;
            }

            let byte = app.buffer.read(byte_off, 1).map(|b| b[0]).unwrap_or(0);
            let is_cursor = byte_off == cursor_offset;
            let in_sel = app.cursor.in_selection(byte_off);

            let cursor_style = Style::default()
                .fg(Color::Black)
                .bg(match app.mode {
                    Mode::Insert => Color::Green,
                    _ => Color::Yellow,
                })
                .add_modifier(Modifier::BOLD);
            let normal_style = Style::default();
            let sel_style = Style::default().bg(Color::DarkGray);

            if is_cursor && app.nibble_mode {
                let high_digit = byte >> 4;
                let low_digit = byte & 0x0F;
                let (high_style, low_style) = if app.cursor.sub_offset == 0 {
                    (cursor_style, normal_style)
                } else {
                    (normal_style, cursor_style)
                };
                spans.push(Span::styled(format!("{:X}", high_digit), high_style));
                spans.push(Span::styled(format!("{:X}", low_digit), low_style));
                spans.push(Span::styled(" ", normal_style));
            } else if is_cursor {
                spans.push(Span::styled(format!("{:02X} ", byte), cursor_style));
            } else if in_sel {
                spans.push(Span::styled(format!("{:02X} ", byte), sel_style));
            } else {
                spans.push(Span::styled(format!("{:02X} ", byte), normal_style));
            }
        }

        spans.push(Span::raw(" "));

        for col in 0..bytes_per_row {
            let byte_off = row_offset + col as u64;
            if byte_off >= app.buffer.len() {
                spans.push(Span::raw(" "));
                continue;
            }
            let byte = app.buffer.read(byte_off, 1).map(|b| b[0]).unwrap_or(0);
            let is_cursor = byte_off == cursor_offset;
            let in_sel = app.cursor.in_selection(byte_off);
            let ch = if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '.'
            };

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if in_sel {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            spans.push(Span::styled(ch.to_string(), style));
        }

        let line_style = if row == current_row {
            Style::default().bg(Color::Rgb(30, 30, 50))
        } else {
            Style::default()
        };
        lines.push(Line::from(spans).style(line_style));
    }

    frame.render_widget(Paragraph::new(lines), area);
}
