//! Unit tests for stacked document open and document store.

use std::path::{Path, PathBuf};

use aura_editor::{
    DocumentStore, ExtensionRuntime, OpenError, DOC_TYPE_AURA_JSON, DOC_TYPE_IMP_GRAPH,
    DOC_TYPE_JSON,
};
use imp_core_types::Graph;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn loads_single_graph_fixture() {
    let runtime = ExtensionRuntime::load_bundled();
    let opened = runtime.open_document(fixture("sine.json")).expect("sine");
    assert_eq!(
        opened.type_chain,
        vec![
            DOC_TYPE_JSON.to_string(),
            DOC_TYPE_AURA_JSON.to_string(),
            DOC_TYPE_IMP_GRAPH.to_string()
        ]
    );
    let graph = opened
        .payload
        .downcast_ref::<Graph>()
        .expect("imp-graph tip");
    assert!(graph.nodes.contains_key("osc"));
    assert!(graph.edges.contains_key("osc_to_out"));
}

#[test]
fn rejects_graph_library_with_typed_error() {
    let runtime = ExtensionRuntime::load_bundled();
    let err = runtime
        .open_document(fixture("instruments.json"))
        .unwrap_err();
    assert!(
        matches!(err, OpenError::GraphLibraryNotSupported { .. }),
        "expected GraphLibraryNotSupported, got {err:?}"
    );
}

#[test]
fn rejects_invalid_json_with_typed_error() {
    let runtime = ExtensionRuntime::load_bundled();
    let err = runtime.open_document(fixture("invalid.json")).unwrap_err();
    // invalid.json is not valid JSON → no root document type matches.
    assert!(
        matches!(err, OpenError::Unrecognized { .. }),
        "expected Unrecognized, got {err:?}"
    );
}

#[test]
fn rejects_missing_file_with_io_error() {
    let runtime = ExtensionRuntime::load_bundled();
    let err = runtime
        .open_document(fixture("does-not-exist.json"))
        .unwrap_err();
    assert!(
        matches!(err, OpenError::Io { .. }),
        "expected Io, got {err:?}"
    );
}

#[test]
fn unknown_aura_type_errors() {
    let runtime = ExtensionRuntime::load_bundled();
    let err = runtime
        .open_document_bytes(
            Path::new("unknown.json"),
            br#"{ "type": "not-a-real-type", "nodes": {}, "edges": {} }"#,
        )
        .unwrap_err();
    assert!(
        matches!(err, OpenError::UnknownAuraType { ref type_id, .. } if type_id == "not-a-real-type"),
        "expected UnknownAuraType, got {err:?}"
    );
}

#[test]
fn plain_json_without_type_stops_at_json() {
    let runtime = ExtensionRuntime::load_bundled();
    let opened = runtime
        .open_document_bytes(Path::new("plain.json"), br#"{ "hello": 1 }"#)
        .expect("plain json");
    assert_eq!(opened.type_chain, vec![DOC_TYPE_JSON.to_string()]);
}

#[test]
fn open_select_close_tabs() {
    let mut store = DocumentStore::new();
    let sine = Graph {
        nodes: Default::default(),
        edges: Default::default(),
    };
    let other = Graph {
        nodes: Default::default(),
        edges: Default::default(),
    };

    let a = store.open_or_activate(PathBuf::from("/tmp/a.json"), sine.clone());
    let b = store.open_or_activate(PathBuf::from("/tmp/b.json"), other);
    assert_eq!(store.documents().len(), 2);
    assert_eq!(store.active_id(), Some(b));

    store.select(a);
    assert_eq!(store.active_id(), Some(a));

    store.close(a);
    assert_eq!(store.documents().len(), 1);
    assert_eq!(store.active_id(), Some(b));

    store.close(b);
    assert!(store.documents().is_empty());
    assert_eq!(store.active_id(), None);
}
