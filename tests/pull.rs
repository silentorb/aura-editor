//! Miro-profile pull → ModelGraph by NodeId (primary open + missing-card resolve).

use std::fs;
use std::path::PathBuf;

use aura_editor::{App, Bindings, DocumentOrigin, GraphEdit, Message};
use imp_core_types::Graph;

const SINE_OSC_NODE: &str = "0033AFBGAY03CEW3Z5AP3J086B";
const NOTATION_TIP_ID: &str = "01JNOTATIONTIP000000000001";
const DANGLING_OUT_NODE: &str = "OUTNODEID00000000000000001";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_content_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "aura-editor-host-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("mkdir");
    dir
}

#[test]
fn pulls_imp_graph_into_model_graph_by_node_id() {
    let root = temp_content_root();
    fs::copy(fixture("sine.json"), root.join("sine.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let id = app.open_pulled_node_tab(SINE_OSC_NODE).expect("pull");

    let doc = app.documents().get(id).expect("doc");
    let graph = doc.payload.graph().expect("imp graph");
    assert!(graph
        .nodes
        .values()
        .any(|node| node.title.as_deref() == Some("osc")));
    assert_eq!(
        app.documents().origin(id),
        Some(&DocumentOrigin::Pulled {
            content_root: root.clone(),
            node_id: SINE_OSC_NODE.into(),
            mir_record_id: "sine.json".into(),
        })
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pulls_notation_into_embedded_model_by_tip_id() {
    let root = temp_content_root();
    fs::copy(fixture("notation.json"), root.join("notation.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let id = app.open_pulled_node_tab(NOTATION_TIP_ID).expect("pull");

    let doc = app.documents().get(id).expect("doc");
    let notation = doc.payload.notation().expect("notation");
    assert_eq!(notation.events.len(), 2);
    let model = doc.payload.model().expect("model");
    assert!(model.graph.nodes.contains_key(NOTATION_TIP_ID));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pull_preserves_dangling_edge_topology() {
    let root = temp_content_root();
    fs::copy(fixture("dangling-edge.json"), root.join("dangling.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let id = app.open_pulled_node_tab(DANGLING_OUT_NODE).expect("pull");

    let graph: &Graph = app
        .documents()
        .get(id)
        .expect("doc")
        .payload
        .graph()
        .expect("graph");
    assert!(graph.nodes.contains_key(DANGLING_OUT_NODE));
    assert!(!graph.nodes.contains_key("ghost"));
    let edge = graph.edges.get("ghost_to_out").expect("edge");
    assert_eq!(edge.from.node, "ghost");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pulled_reopen_dedupes_same_mir_blob() {
    let root = temp_content_root();
    fs::copy(fixture("sine.json"), root.join("sine.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let id1 = app.open_pulled_node_tab(SINE_OSC_NODE).expect("pull");
    // Different node in the same file → same tab.
    let id2 = app
        .open_pulled_node_tab("0033AFBGAW03BV387HWA48PA51")
        .expect("reopen");
    assert_eq!(id1, id2);
    assert_eq!(app.documents().documents().len(), 1);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn open_pulled_node_id_message_opens_tab() {
    let root = temp_content_root();
    fs::copy(fixture("sine.json"), root.join("sine.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let _ = app.update(Message::OpenPulledNodeId(SINE_OSC_NODE.into()));
    assert_eq!(app.documents().documents().len(), 1);
    assert!(app
        .documents()
        .active_document()
        .and_then(|doc| doc.payload.graph())
        .is_some());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn open_indexes_notation_tip_edge_as_remote_without_proxy() {
    let root = temp_content_root();
    fs::copy(fixture("notation.json"), root.join("arp.json")).expect("copy notation");
    let graph_json = format!(
        r#"{{
          "type": "imp-graph",
          "nodes": {{
            "loop1": {{ "id": "loop1", "type": "loop", "inputs": {{}}, "title": "loop" }}
          }},
          "edges": {{
            "arp_to_loop": {{
              "from": {{ "node": "{NOTATION_TIP_ID}", "port": "score" }},
              "to": {{ "node": "loop1", "port": "value" }}
            }}
          }}
        }}"#
    );
    fs::write(root.join("parent.json"), graph_json).expect("write graph");

    let runtime = aura_editor::ExtensionRuntime::load_bundled();
    let opened = runtime
        .open_document(root.join("parent.json"))
        .expect("open");
    let mut store = aura_editor::DocumentStore::new();
    let id = store
        .open_or_activate_stacked(root.join("parent.json"), opened)
        .expect("activate");
    let graph = store.get(id).expect("doc").payload.graph().expect("graph");
    assert!(!graph.nodes.contains_key(NOTATION_TIP_ID));
    let registry = aura_imp::aura_registry().expect("registry");
    let remotes = aura_editor::indexed_remote_nodes(graph, &root, &registry).expect("index");
    let remote = remotes.get(NOTATION_TIP_ID).expect("remote notation tip");
    assert_eq!(remote.display.node_type, aura_editor::NOTATION_NODE_TYPE);
    assert_eq!(remote.display.title.as_deref(), Some("arp"));
    assert!(remote.ports.outputs.iter().any(|p| p == "score"));
    assert!(!store.get(id).expect("doc").dirty);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn selecting_remote_notation_fills_title_buffer_from_document() {
    let root = temp_content_root();
    fs::copy(fixture("notation.json"), root.join("arp.json")).expect("copy notation");
    let graph_json = format!(
        r#"{{
          "type": "imp-graph",
          "nodes": {{
            "loop1": {{ "id": "loop1", "type": "loop", "inputs": {{}}, "title": "loop" }}
          }},
          "edges": {{
            "arp_to_loop": {{
              "from": {{ "node": "{NOTATION_TIP_ID}", "port": "score" }},
              "to": {{ "node": "loop1", "port": "value" }}
            }}
          }}
        }}"#
    );
    fs::write(root.join("parent.json"), graph_json).expect("write graph");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let id = app.open_pulled_node_tab("loop1").expect("open parent");
    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::SelectNode {
            node_id: Some(NOTATION_TIP_ID.into()),
        },
    });
    assert_eq!(app.graph_ui().selected_node.as_deref(), Some(NOTATION_TIP_ID));
    assert_eq!(app.graph_ui().title_buffer, "arp");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn open_does_not_hydrate_unindexed_imp_graph_endpoints() {
    let root = temp_content_root();
    fs::copy(fixture("dangling-edge.json"), root.join("dangling.json")).expect("copy");
    // ghost tip is not present under root — stays missing (red).

    let runtime = aura_editor::ExtensionRuntime::load_bundled();
    let opened = runtime
        .open_document(root.join("dangling.json"))
        .expect("open");
    let mut store = aura_editor::DocumentStore::new();
    let id = store
        .open_or_activate_stacked(root.join("dangling.json"), opened)
        .expect("activate");
    let graph = store.get(id).expect("doc").payload.graph().expect("graph");
    assert!(!graph.nodes.contains_key("ghost"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn open_node_resolves_missing_card_into_local_map() {
    let root = temp_content_root();
    fs::copy(fixture("dangling-edge.json"), root.join("dangling.json")).expect("copy");
    fs::copy(fixture("ghost-node.json"), root.join("ghost.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_workspace_in_memory(&root).expect("session");
    let id = app.open_pulled_node_tab(DANGLING_OUT_NODE).expect("open dangling");
    {
        let graph = app
            .documents()
            .get(id)
            .expect("doc")
            .payload
            .graph()
            .expect("graph");
        assert!(!graph.nodes.contains_key("ghost"));
    }

    let _ = app.update(Message::Graph {
        document_id: id,
        edit: GraphEdit::OpenNode {
            node_id: "ghost".into(),
        },
    });

    let graph = app
        .documents()
        .get(id)
        .expect("doc")
        .payload
        .graph()
        .expect("graph");
    let ghost = graph.nodes.get("ghost").expect("resolved into local map");
    assert_eq!(ghost.node_type, "time");
    assert_eq!(ghost.title.as_deref(), Some("ghost clock"));
    // Resolve also opens the tip that owns the node when a view maps.
    assert!(app.documents().documents().len() >= 1);
    let _ = fs::remove_dir_all(&root);
}
