//! Native application menu bar (File / Edit / View / Play).
//!
//! Labels and item ids mirror the Iced shell in `aura-editor/src/menu_bar.rs`.
//! Command dispatch is deferred; clicks are intentionally ignored for now.

use tauri::menu::{Menu, MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// Command id strings aligned with Iced / `aura_interfaces` (duplicated so the
/// experimental host stays decoupled until command wiring lands).
const CMD_DOCUMENT_OPEN_WORKSPACE: &str = "document.open-workspace";
const CMD_DOCUMENT_OPEN: &str = "document.open";
const CMD_DOCUMENT_OPEN_FILE: &str = "document.open-file";
const CMD_DOCUMENT_NEW_NOTATION: &str = "document.new.notation";
const CMD_DOCUMENT_NEW_GRAPH: &str = "document.new.graph";
const CMD_DOCUMENT_SAVE: &str = "document.save";
const CMD_DOCUMENT_CLOSE_ACTIVE: &str = "document.close_active";
const CMD_PLAY_START: &str = "play.start";
const CMD_PLAY_STOP: &str = "play.stop";

/// Builds the global File / Edit / View / Play menu bar.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let file = SubmenuBuilder::new(app, "File")
        .text(CMD_DOCUMENT_OPEN_WORKSPACE, "Open Workspace…")
        .text(CMD_DOCUMENT_OPEN, "Open…")
        .text(CMD_DOCUMENT_OPEN_FILE, "Open File…")
        .text(CMD_DOCUMENT_NEW_NOTATION, "New notation")
        .text(CMD_DOCUMENT_NEW_GRAPH, "New graph")
        .text(CMD_DOCUMENT_SAVE, "Save")
        .text(CMD_DOCUMENT_CLOSE_ACTIVE, "Close")
        .build()?;

    // Edit / View stay as labeled roots until they have items (Iced parity).
    let edit = SubmenuBuilder::new(app, "Edit").build()?;
    let view = SubmenuBuilder::new(app, "View").build()?;

    let play = SubmenuBuilder::new(app, "Play")
        .text(CMD_PLAY_START, "Start")
        .text(CMD_PLAY_STOP, "Stop")
        .build()?;

    MenuBuilder::new(app)
        .item(&file)
        .item(&edit)
        .item(&view)
        .item(&play)
        .build()
}
