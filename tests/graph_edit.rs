//! Graph-dag editing: mutate, dirty, save, and UI prefs.

use std::path::PathBuf;

use aura_editor::{
    App, Bindings, ConnectionMode, GraphChromeMessage, GraphEdit, GraphUiEntry, GraphUiState,
    Message, CMD_DOCUMENT_NEW_GRAPH, CMD_GRAPH_ADD_NODE, CommandId,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn test_app() -> App {
    App::with_bindings(Bindings::empty())
}

#[test]
fn open_sine_erase_node_marks_dirty() {
    let mut app = test_app();
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let id = app.documents().active_id().unwrap();
    assert!(!app.documents().active_document().unwrap().dirty);

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::EraseNode {
            node_id: String::from("freq"),
        },
    });

    let doc = app.documents().active_document().unwrap();
    assert!(doc.dirty);
    assert!(!doc.payload.graph().unwrap().nodes.contains_key("freq"));
}

#[test]
fn add_node_command_opens_dialog_and_confirm_inserts() {
    let mut app = test_app();
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_NEW_GRAPH)));
    let id = app.documents().active_id().unwrap();

    let _ = app.update(Message::Command(CommandId::new(CMD_GRAPH_ADD_NODE)));
    let _ = app.update(Message::GraphChrome(GraphChromeMessage::SetAddNodeType(
        String::from("sine"),
    )));
    let _ = app.update(Message::GraphChrome(GraphChromeMessage::AddNodeConfirm));
    let _ = app.update(Message::Command(CommandId::new(CMD_GRAPH_ADD_NODE)));
    let _ = app.update(Message::GraphChrome(GraphChromeMessage::AddNodeConfirm));

    let graph = app
        .documents()
        .active_document()
        .unwrap()
        .payload
        .graph()
        .unwrap();
    assert!(graph.nodes.contains_key("sine"));
    assert!(graph.nodes.contains_key("sine_2"));
    assert_eq!(app.documents().active_id(), Some(id));
}

#[test]
fn simple_connect_auto_pairs_ports() {
    let mut app = test_app();
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let id = app.documents().active_id().unwrap();

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::EraseEdgesBetween {
            a: String::from("freq"),
            b: String::from("osc"),
        },
    });

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::ConnectNodes {
            from: String::from("freq"),
            to: String::from("osc"),
        },
    });

    let graph = app
        .documents()
        .active_document()
        .unwrap()
        .payload
        .graph()
        .unwrap();
    assert!(graph.edges.values().any(|edge| {
        edge.from.node == "freq"
            && edge.from.port == "value"
            && edge.to.node == "osc"
            && edge.to.port == "frequency"
    }));
}

#[test]
fn disconnect_port_removes_incident_edges() {
    let mut app = test_app();
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let id = app.documents().active_id().unwrap();

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::DisconnectPort {
            node_id: String::from("freq"),
            port_id: String::from("value"),
            is_output: true,
        },
    });

    let graph = app
        .documents()
        .active_document()
        .unwrap()
        .payload
        .graph()
        .unwrap();
    assert!(!graph.edges.values().any(|edge| {
        edge.from.node == "freq" && edge.from.port == "value"
    }));
    assert!(app.documents().active_document().unwrap().dirty);
}

#[test]
fn connection_mode_persists_in_entry_shape() {
    let state = GraphUiState {
        connection_mode: ConnectionMode::Precise,
        add_node_type: String::from("add"),
        selected_node: None,
        selected_edge: None,
        connect_draft: None,
        rename_buffer: String::new(),
    };
    let entry = state.to_entry();
    assert_eq!(entry.connection_mode, "precise");
    assert_eq!(entry.add_node_type, "add");
    let restored = GraphUiState::from_entry(&entry, "sine");
    assert_eq!(restored.connection_mode, ConnectionMode::Precise);
    assert_eq!(restored.add_node_type, "add");
}

#[test]
fn graph_ui_entry_defaults() {
    let defaults = GraphUiEntry::new("sine");
    assert_eq!(defaults.connection_mode, "simple");
    assert_eq!(defaults.add_node_type, "sine");
}

#[test]
fn rename_input_does_not_commit_until_matching_generation() {
    let mut app = test_app();
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let id = app.documents().active_id().unwrap();

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::SelectNode {
            node_id: Some(String::from("freq")),
        },
    });
    let _ = app.update(Message::GraphChrome(GraphChromeMessage::RenameInput(
        String::from("freq2"),
    )));

    let doc = app.documents().active_document().unwrap();
    assert!(!doc.dirty);
    assert!(doc.payload.graph().unwrap().nodes.contains_key("freq"));
    assert!(!doc.payload.graph().unwrap().nodes.contains_key("freq2"));

    // SelectNode bumps generation to 1; RenameInput bumps to 2.
    let _ = app.update(Message::CommitGraphRename { generation: 1 });
    let doc = app.documents().active_document().unwrap();
    assert!(!doc.dirty);
    assert!(doc.payload.graph().unwrap().nodes.contains_key("freq"));

    let _ = app.update(Message::CommitGraphRename { generation: 2 });
    let doc = app.documents().active_document().unwrap();
    assert!(doc.dirty);
    assert!(!doc.payload.graph().unwrap().nodes.contains_key("freq"));
    assert!(doc.payload.graph().unwrap().nodes.contains_key("freq2"));
    assert_eq!(app.graph_ui().selected_node.as_deref(), Some("freq2"));
}

#[test]
fn rename_commit_applies_immediately_and_invalidates_pending() {
    let mut app = test_app();
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let id = app.documents().active_id().unwrap();

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::SelectNode {
            node_id: Some(String::from("freq")),
        },
    });
    let _ = app.update(Message::GraphChrome(GraphChromeMessage::RenameInput(
        String::from("freq2"),
    )));
    let _ = app.update(Message::GraphChrome(GraphChromeMessage::RenameCommit));

    let doc = app.documents().active_document().unwrap();
    assert!(doc.dirty);
    assert!(doc.payload.graph().unwrap().nodes.contains_key("freq2"));

    // Stale debounce from RenameInput (generation 2) must not re-apply after Enter.
    let _ = app.update(Message::CommitGraphRename { generation: 2 });
    assert!(app
        .documents()
        .active_document()
        .unwrap()
        .payload
        .graph()
        .unwrap()
        .nodes
        .contains_key("freq2"));
}

#[test]
fn new_graph_save_round_trip() {
    let dir = std::env::temp_dir().join(format!(
        "aura-editor-graph-edit-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("scratch.json");

    let mut app = test_app();
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_NEW_GRAPH)));
    let id = app.documents().active_id().unwrap();
    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::AddNode,
    });
    let _ = app.update(Message::SaveToPath {
        document_id: id,
        path: path.clone(),
    });

    assert!(!app.documents().active_document().unwrap().dirty);
    let text = std::fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["type"], "imp-graph");
    assert!(value["nodes"].as_object().unwrap().contains_key("sine"));

    let mut app2 = test_app();
    let _ = app2.update(Message::OpenPath(path));
    assert_eq!(app2.documents().documents().len(), 1);
    assert!(app2
        .documents()
        .active_document()
        .unwrap()
        .payload
        .graph()
        .unwrap()
        .nodes
        .contains_key("sine"));

    let _ = std::fs::remove_dir_all(&dir);
}
