//! Aura Editor binary entrypoint.

// Suppress the console window on Windows; logs go to ./logs instead.
#![cfg_attr(windows, windows_subsystem = "windows")]

use aura_editor::App;
use native_theme_iced::styles;

fn main() -> iced::Result {
    aura_editor::init_logging();
    tracing::info!("starting Aura Editor");

    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(|app: &App| app.theme.clone())
        .style(|app: &App, theme| styles::application(&app.resolved)(theme))
        .run()
}
