//! Unit tests for graph file loading and document store.

use std::path::PathBuf;

use aura_editor::{
    load_graph_from_path, load_graph_from_str, DocumentStore, OpenError,
};
use imp_core_types::Graph;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn loads_single_graph_fixture() {
    let graph = load_graph_from_path(fixture("sine.json")).expect("sine graph");
    assert!(graph.nodes.contains_key("osc"));
    assert!(graph.edges.contains_key("osc_to_out"));
}

#[test]
fn rejects_graph_library_with_typed_error() {
    let err = load_graph_from_path(fixture("instruments.json")).unwrap_err();
    assert!(
        matches!(err, OpenError::GraphLibraryNotSupported { .. }),
        "expected GraphLibraryNotSupported, got {err:?}"
    );
}

#[test]
fn rejects_invalid_json_with_typed_error() {
    let err = load_graph_from_path(fixture("invalid.json")).unwrap_err();
    assert!(
        matches!(err, OpenError::Json { .. }),
        "expected Json, got {err:?}"
    );
}

#[test]
fn rejects_missing_file_with_io_error() {
    let err = load_graph_from_path(fixture("does-not-exist.json")).unwrap_err();
    assert!(
        matches!(err, OpenError::Io { .. }),
        "expected Io, got {err:?}"
    );
}

#[test]
fn rejects_non_graph_object_as_invalid_graph() {
    let err = load_graph_from_str(
        PathBuf::from("array.json"),
        "[1, 2, 3]",
    )
    .unwrap_err();
    assert!(
        matches!(err, OpenError::InvalidGraph { .. }),
        "expected InvalidGraph, got {err:?}"
    );
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

    let again = store.open_or_activate(PathBuf::from("/tmp/a.json"), sine);
    assert_eq!(again, a);
    assert_eq!(store.documents().len(), 2);

    store.close(a);
    assert_eq!(store.documents().len(), 1);
    assert_eq!(store.active_id(), Some(b));
    assert_eq!(store.active_document().unwrap().title, "b");

    store.close(b);
    assert!(store.documents().is_empty());
    assert_eq!(store.active_id(), None);
}
