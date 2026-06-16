use std::path::PathBuf;

mod app;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let mut app = app::App::new();

    if let Some(path) = std::env::args_os().nth(1) {
        app.open_file(&PathBuf::from(path))
            .unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
    }

    while !app.quit_requested {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            app.handle_key(key).unwrap_or_else(|e| {
                app.status_message = format!("Error: {}", e);
            });
        }
    }

    ratatui::restore();
    Ok(())
}
