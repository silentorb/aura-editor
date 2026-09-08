//! Headless shell tests for open / close document flow.

use std::path::PathBuf;

use aura_editor::{App, Message};
use iced_test::simulator;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn empty_state_prompts_to_open() {
    let app = App::new();
    let mut ui = simulator(app.view());

    assert!(ui.find("Open…").is_ok());
    assert!(ui.find("Open a graph file to begin").is_ok());
}

#[test]
fn open_path_creates_tab_and_close_restores_empty_state() {
    let mut app = App::new();
    let path = fixture("sine.json");

    let _ = app.update(Message::OpenPath(path.clone()));
    assert_eq!(app.documents().documents().len(), 1);
    assert_eq!(app.documents().active_document().unwrap().title, "sine");
    assert_eq!(app.title(), "Aura Editor — sine");
    assert!(app.last_error().is_none());

    let mut ui = simulator(app.view());
    assert!(ui.find("[sine]").is_ok(), "active tab title should be visible");
    assert!(ui.find("Ready").is_ok());
    drop(ui);

    let id = app.documents().active_id().unwrap();
    let _ = app.update(Message::CloseTab(id));
    assert!(app.documents().documents().is_empty());

    let mut ui = simulator(app.view());
    assert!(ui.find("Open a graph file to begin").is_ok());
}

#[test]
fn open_graph_library_surfaces_typed_error() {
    let mut app = App::new();
    let _ = app.update(Message::OpenPath(fixture("instruments.json")));

    assert!(app.documents().documents().is_empty());
    assert!(matches!(
        app.last_error(),
        Some(aura_editor::OpenError::GraphLibraryNotSupported { .. })
    ));

    let status = app.status();
    assert!(
        status.contains("graph library not supported yet"),
        "status should explain GraphLibrary rejection, got {status}"
    );

    let mut ui = simulator(app.view());
    assert!(
        ui.find(status.as_str()).is_ok(),
        "error status should be visible in the shell"
    );
}
