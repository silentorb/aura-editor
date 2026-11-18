mod menu;
mod open;

use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Serialize)]
struct CommandPayload {
    id: String,
}

#[tauri::command]
fn ping() -> String {
    "pong".into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .menu(menu::build)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref().to_string();
            if let Err(err) = app.emit("editor://command", CommandPayload { id }) {
                eprintln!("failed to emit editor://command: {err}");
            }
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            open::pick_and_open_document,
            open::open_document_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
