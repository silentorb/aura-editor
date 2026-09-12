//! Aura Editor binary entrypoint.

// Suppress the console window on Windows; logs go to ./logs instead.
#![cfg_attr(windows, windows_subsystem = "windows")]

use aura_editor::App;

fn main() -> iced::Result {
    aura_editor::init_logging();
    tracing::info!("starting Aura Editor");

    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .run()
}
