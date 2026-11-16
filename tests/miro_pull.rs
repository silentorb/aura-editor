//! Miro pull → ModelGraph (parallel workflow; not menu-attached).

use std::fs;
use std::path::PathBuf;

use aura_editor::{
    App, Bindings, DocumentOrigin, NOTATION_SCORE_NODE_ID,
};
use imp_core_types::Graph;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_content_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "aura-editor-miro-{}-{}",
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
fn pulls_imp_graph_into_model_graph() {
    let root = temp_content_root();
    fs::copy(fixture("sine.json"), root.join("sine.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_miro_workspace_in_memory(&root).expect("session");
    let id = app.open_miro_record_tab("sine.json").expect("pull");

    let doc = app.documents().get(id).expect("doc");
    let graph = doc.payload.graph().expect("imp graph");
    assert!(graph
        .nodes
        .values()
        .any(|node| node.title.as_deref() == Some("osc")));
    assert_eq!(
        app.documents().origin(id),
        Some(&DocumentOrigin::Miro {
            content_root: root.clone(),
            record_id: "sine.json".into(),
        })
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pulls_notation_into_embedded_model() {
    let root = temp_content_root();
    fs::copy(fixture("notation.json"), root.join("notation.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_miro_workspace_in_memory(&root).expect("session");
    let id = app.open_miro_record_tab("notation.json").expect("pull");

    let doc = app.documents().get(id).expect("doc");
    let notation = doc.payload.notation().expect("notation");
    assert_eq!(notation.events.len(), 2);
    let model = doc.payload.model().expect("model");
    assert!(model.graph.nodes.contains_key(NOTATION_SCORE_NODE_ID));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pull_preserves_dangling_edge_topology() {
    let root = temp_content_root();
    fs::copy(fixture("dangling-edge.json"), root.join("dangling.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_miro_workspace_in_memory(&root).expect("session");
    let id = app.open_miro_record_tab("dangling.json").expect("pull");

    let graph: &Graph = app
        .documents()
        .get(id)
        .expect("doc")
        .payload
        .graph()
        .expect("graph");
    assert!(graph.nodes.contains_key("OUTNODEID00000000000000001"));
    assert!(!graph.nodes.contains_key("ghost"));
    let edge = graph.edges.get("ghost_to_out").expect("edge");
    assert_eq!(edge.from.node, "ghost");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn miro_reopen_dedupes_same_record() {
    let root = temp_content_root();
    fs::copy(fixture("sine.json"), root.join("sine.json")).expect("copy");

    let mut app = App::with_bindings(Bindings::empty());
    app.open_miro_workspace_in_memory(&root).expect("session");
    let id1 = app.open_miro_record_tab("sine.json").expect("pull");
    let id2 = app.open_miro_record_tab("sine.json").expect("reopen");
    assert_eq!(id1, id2);
    assert_eq!(app.documents().documents().len(), 1);
    let _ = fs::remove_dir_all(&root);
}
