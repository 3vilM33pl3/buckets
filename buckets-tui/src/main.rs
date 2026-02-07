// SPDX-License-Identifier: MIT

use std::io;
use std::panic;
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use env_logger::Builder;
use log::LevelFilter;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

mod app;
mod db;
mod event;
mod message;
mod model;
mod views;
mod widgets;

use app::App;
use message::AppMessage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse minimal CLI args
    let verbose = std::env::args().any(|a| a == "-v" || a == "--verbose");

    init_logging(verbose);

    // Bootstrap database
    buckets::bootstrap::bootstrap_database()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Install panic hook that restores the terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Create message channel
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();

    // Create app
    let mut app = App::new(tx.clone());
    app.load_initial_data();

    // Spawn tick timer
    let tick_tx = tx.clone();
    let poll_interval = Duration::from_secs(5);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(poll_interval).await;
            if tick_tx.send(AppMessage::Tick).is_err() {
                break;
            }
        }
    });

    // Main event loop
    let event_timeout = Duration::from_millis(50);
    loop {
        // Draw
        terminal.draw(|f| app.view(f))?;

        // Poll crossterm events
        if let Some(msg) = event::poll_event(event_timeout) {
            app.update(msg);
        }

        // Drain async messages
        while let Ok(msg) = rx.try_recv() {
            app.update(msg);
        }

        if app.model.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn init_logging(verbose: bool) {
    let log_level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };

    Builder::from_default_env().filter_level(log_level).init();
}
