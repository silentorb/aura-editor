//! Graph-dag editing: mutate, dirty, save, and UI prefs.

use std::path::PathBuf;

use aura_editor::{
    add_node, graph_to_aura_json, App, Bindings, ConnectionMode, GraphEdit, GraphTool, GraphUiEntry,
    GraphUiState, Message, CMD_DOCUMENT_NEW_GRAPH, CMD_GRAPH_TOOL_CONNECT, CommandId,
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
fn add_node_uses_unique_id() {
    let mut app = test_app();
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_NEW_GRAPH)));
    let id = app.documents().active_id().unwrap();

    let _ = app.update(Message::GraphChrome(
        aura_editor::GraphChromeMessage::SetAddNodeType(String::from("sine")),
    ));
    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::AddNode,
    });
    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::AddNode,
    });

    let graph = app.documents().active_document().unwrap().payload.graph().unwrap();
    assert!(graph.nodes.contains_key("sine"));
    assert!(graph.nodes.contains_key("sine_2"));
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

    let _ = app.update(Message::Command(CommandId::new(CMD_GRAPH_TOOL_CONNECT)));
    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::ConnectNodes {
            from: String::from("freq"),
            to: String::from("osc"),
        },
    });

    let graph = app.documents().active_document().unwrap().payload.graph().unwrap();
    assert!(graph.edges.values().any(|edge| {
        edge.from.node == "freq"
            && edge.from.port == "value"
            && edge.to.node == "osc"
            && edge.to.port == "frequency"
    }));
}

#[test]
fn connection_mode_persists_in_entry_shape() {
    let state = GraphUiState {
        connection_mode: ConnectionMode::Precise,
        tool: GraphTool::Connect,
        add_node_type: String::from("add"),
        selected_node: None,
        selected_edge: None,
        connect_draft: None,
        rename_buffer: String::new(),
    };
    let entry = state.to_entry();
    assert_eq!(entry.connection_mode, "precise");
    assert_eq!(entry.active_tool, "connect");
    assert_eq!(entry.add_node_type, "add");
    let restored = GraphUiState::from_entry(&entry, "sine");
    assert_eq!(restored.connection_mode, ConnectionMode::Precise);
    assert_eq!(restored.tool, GraphTool::Connect);
}

#[test]
fn graph_ui_entry_defaults() {
    let defaults = GraphUiEntry::new("sine");
    assert_eq!(defaults.active_tool, "select");
    assert_eq!(defaults.connection_mode, "simple");
    assert_eq!(defaults.add_node_type, "sine");
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

#[test]
fn graph_to_aura_json_helper_used_by_save() {
    let mut graph = imp_core_types::Graph {
        nodes: Default::default(),
        edges: Default::default(),
    };
    add_node(&mut graph, "out", "output").unwrap();
    let json = graph_to_aura_json(&graph).unwrap();
    assert!(json.contains("imp-graph"));
}
