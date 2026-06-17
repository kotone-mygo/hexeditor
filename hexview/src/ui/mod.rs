use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::app::App;

mod command_bar;
mod config_view;
mod help_view;
mod hex_view;
mod status_bar;
mod tabs;

pub fn render(app: &App, frame: &mut Frame) {
    let show_tabs = app.tabs.len() > 1;

    let mut constraints = Vec::new();
    if show_tabs {
        constraints.push(Constraint::Length(1));
    }
    if app.show_help {
        constraints.push(Constraint::Min(1));
        constraints.push(Constraint::Length(13));
    } else {
        constraints.push(Constraint::Min(1));
    }
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let mut idx = 0;
    if show_tabs {
        tabs::render_tabs(frame, chunks[idx], app);
        idx += 1;
    }
    if app.show_help {
        hex_view::render_hex_view(frame, chunks[idx], app);
        idx += 1;
        help_view::render_help_view(frame, chunks[idx], app);
        idx += 1;
    } else if app.show_config {
        config_view::render_config_view(frame, chunks[idx], app);
        idx += 1;
    } else {
        hex_view::render_hex_view(frame, chunks[idx], app);
        idx += 1;
    }
    command_bar::render_command_bar(frame, chunks[idx], app);
    idx += 1;
    status_bar::render_status_bar(frame, chunks[idx], app);
}
