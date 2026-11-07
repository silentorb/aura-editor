//! Headless shell tests for open / close document flow.

use std::path::PathBuf;

use aura_editor::{App, Bindings, GraphEdit, Message};
use iced_test::simulator;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn test_app() -> App {
    App::with_bindings(Bindings::empty())
}

#[test]
fn empty_state_prompts_to_open() {
    let app = test_app();
    let mut ui = simulator(app.view());

    assert!(ui.find("File").is_ok(), "global File menu root should be visible");
    assert!(ui
        .find("Open a file or create a notation to begin")
        .is_ok());
}

#[test]
fn default_shell_shows_dockable_panel_titles() {
    let app = test_app();
    let mut ui = simulator(app.view());

    assert!(ui.find("Status").is_ok(), "status pane title should be visible");
    assert!(ui.find("Toolbar").is_ok(), "toolbar pane title should be visible");
    assert!(
        ui.find("Documents").is_ok(),
        "documents pane title should be visible"
    );
    assert!(
        ui.find("Properties").is_ok(),
        "properties pane title should be visible"
    );
    assert_eq!(app.shell_panes().len(), 4);
}

#[test]
fn properties_pane_shows_node_editors_when_selected() {
    let mut app = test_app();
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let id = app.documents().active_id().unwrap();

    let mut ui = simulator(app.view());
    assert!(
        ui.find("Select a node to edit properties").is_ok(),
        "empty properties state before selection"
    );
    assert!(
        ui.find("Id:").is_err(),
        "Id editor should not appear without a selected node"
    );
    drop(ui);

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::SelectNode {
            node_id: Some(String::from("freq")),
        },
    });
    assert_eq!(
        app.graph_ui().selected_node.as_deref(),
        Some("freq")
    );

    let mut ui = simulator(app.view());
    assert!(ui.find("Node: freq").is_ok());
    assert!(ui.find("Id:").is_ok(), "Properties should show Id label");
    assert!(ui.find("Rename").is_ok(), "Properties should show Rename");
    assert!(ui.find("Type:").is_ok(), "Properties should show Type label");
}

#[test]
fn open_path_creates_tab_and_close_restores_empty_state() {
    let mut app = test_app();
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
    assert!(ui
        .find("Open a file or create a notation to begin")
        .is_ok());
}

#[test]
fn open_graph_library_surfaces_typed_error() {
    let mut app = test_app();
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
