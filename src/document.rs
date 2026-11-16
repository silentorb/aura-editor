//! Open documents and Aura tip payloads for the multi-document shell.
//!
//! Session tab/path/dirty bookkeeping lives in Imp Lab’s [`imp_documents`].
//! On-disk Aura JSON formats are unchanged. In memory, editable tips are always a
//! [`ModelGraph`]: Imp graphs load as their nodes/edges; notation documents load as a
//! single nullary `external_score` node with Aura-owned embedded notation data.

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use aura_ext_notation::Notation;
use aura_integration::authoring_path_for_mir_record;
use imp_core_types::{Graph, Node, NodeId};
use serde_json::Value;

use crate::extensions::StackedOpen;

/// Legacy placeholder NodeId for untitled / pre-tip-id notation documents.
///
/// Miro-indexed notation tips carry a stable tip `id` instead; do not use this as a
/// product open key.
pub const NOTATION_SCORE_NODE_ID: &str = "score";

/// Imp node type for notation-document nullary literals (host-provided score).
pub const NOTATION_SCORE_NODE_TYPE: &str = "external_score";

pub use imp_documents::DocumentId;

/// How an open document was materialized into app memory.
///
/// Filesystem opens and Miro pulls are parallel workflows. Miro tabs dedupe by
/// `(content_root, mir_record_id)`; filesystem tabs dedupe by path. Product Miro
/// identity is [`DocumentOrigin::Miro::node_id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentOrigin {
    /// Loaded (or saved) as a filesystem path.
    Filesystem {
        /// Absolute or user-supplied path.
        path: PathBuf,
    },
    /// Pulled from a Miro SQLite projection for a content root.
    Miro {
        /// Text authoring root for the Miro session.
        content_root: PathBuf,
        /// Product NodeId used to open / resolve this tab.
        node_id: String,
        /// Opaque Mir record id (host text-store key) for dedupe and Save path.
        mir_record_id: String,
    },
}

impl DocumentOrigin {
    /// Filesystem path used for Save / neighbor hints.
    pub fn authoring_path(&self) -> PathBuf {
        match self {
            Self::Filesystem { path } => path.clone(),
            Self::Miro {
                content_root,
                mir_record_id,
                ..
            } => authoring_path_for_mir_record(content_root, mir_record_id),
        }
    }
}

/// Aura-owned structured data attached to a model node (not Imp `PrimitiveValue`).
#[derive(Debug, Clone)]
pub enum EmbeddedPayload {
    /// Hand-authored notation backing a nullary score node.
    Notation(Notation),
}

/// In-memory editable graph: Imp nodes/edges plus optional Aura embedded payloads.
#[derive(Debug, Clone)]
pub struct ModelGraph {
    /// Imp graph structure (nodes and edges).
    pub graph: Graph,
    /// Per-node Aura payloads (e.g. embedded notation on a nullary score node).
    pub embedded: BTreeMap<NodeId, EmbeddedPayload>,
}

impl ModelGraph {
    /// Wraps an Imp graph with no embedded payloads.
    pub fn from_imp_graph(graph: Graph) -> Self {
        Self {
            graph,
            embedded: BTreeMap::new(),
        }
    }

    /// Builds a one-node model for a notation document tip.
    ///
    /// Uses the tip `id` when present; otherwise [`NOTATION_SCORE_NODE_ID`] for
    /// untitled / legacy documents (not a Miro open key).
    pub fn from_notation(notation: Notation) -> Self {
        let node_id = notation.tip_node_id().to_string();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            node_id.clone(),
            Node {
                id: node_id.clone(),
                title: None,
                node_type: NOTATION_SCORE_NODE_TYPE.to_string(),
                type_args: Vec::new(),
                inputs: BTreeMap::new(),
            },
        );
        let mut embedded = BTreeMap::new();
        embedded.insert(node_id, EmbeddedPayload::Notation(notation));
        Self {
            graph: Graph {
                nodes,
                edges: BTreeMap::new(),
            },
            embedded,
        }
    }

    /// Embedded notation on the notation-document nullary node, if present.
    pub fn notation(&self) -> Option<&Notation> {
        self.embedded.values().next().map(|payload| match payload {
            EmbeddedPayload::Notation(notation) => notation,
        })
    }

    /// Mutable embedded notation on the notation-document nullary node, if present.
    pub fn notation_mut(&mut self) -> Option<&mut Notation> {
        self.embedded.values_mut().next().map(|payload| match payload {
            EmbeddedPayload::Notation(notation) => notation,
        })
    }

    /// True when this model is a notation-document tip (nullary embedded score).
    pub fn is_notation_document(&self) -> bool {
        self.notation().is_some()
    }
}

/// Tip payload held for an open session document.
#[derive(Debug, Clone)]
pub enum DocumentPayload {
    /// Node-backed tip (`imp-graph` or `notation` wrapped as a [`ModelGraph`]).
    Model(ModelGraph),
    /// Generic JSON tip (stopped at `json`).
    Json(Value),
}

impl DocumentPayload {
    /// Builds a payload from a stacked tip `Any`, or `None` if unsupported.
    pub fn from_tip(tip_type: &str, payload: Box<dyn Any + Send + Sync>) -> Option<Self> {
        match tip_type {
            aura_interfaces::DOC_TYPE_IMP_GRAPH => payload
                .downcast::<Graph>()
                .ok()
                .map(|graph| Self::Model(ModelGraph::from_imp_graph(*graph))),
            aura_interfaces::DOC_TYPE_NOTATION => payload
                .downcast::<Notation>()
                .ok()
                .map(|notation| Self::Model(ModelGraph::from_notation(*notation))),
            aura_interfaces::DOC_TYPE_JSON => payload
                .downcast::<Value>()
                .ok()
                .map(|value| Self::Json(*value)),
            _ => None,
        }
    }

    /// Node-backed model when this tip is a model document.
    pub fn model(&self) -> Option<&ModelGraph> {
        match self {
            Self::Model(model) => Some(model),
            Self::Json(_) => None,
        }
    }

    /// Mutable node-backed model when this tip is a model document.
    pub fn model_mut(&mut self) -> Option<&mut ModelGraph> {
        match self {
            Self::Model(model) => Some(model),
            Self::Json(_) => None,
        }
    }

    /// Imp graph for graph-document tips (not notation wrappers).
    pub fn graph(&self) -> Option<&Graph> {
        let model = self.model()?;
        if model.is_notation_document() {
            None
        } else {
            Some(&model.graph)
        }
    }

    /// Mutable Imp graph for graph-document tips (not notation wrappers).
    pub fn graph_mut(&mut self) -> Option<&mut Graph> {
        let model = self.model_mut()?;
        if model.is_notation_document() {
            None
        } else {
            Some(&mut model.graph)
        }
    }

    /// Notation when this tip is a notation document (embedded on the nullary node).
    pub fn notation(&self) -> Option<&Notation> {
        self.model()?.notation()
    }

    /// Mutable notation when this tip is a notation document.
    pub fn notation_mut(&mut self) -> Option<&mut Notation> {
        self.model_mut()?.notation_mut()
    }
}

/// An open document held in the editor session.
pub type SessionDocument = imp_documents::SessionDocument<DocumentPayload>;

/// Backward-compatible alias used by older call sites.
pub type GraphDocument = SessionDocument;

/// Ordered collection of open documents plus the active tab, with Aura tip helpers.
#[derive(Debug, Clone, Default)]
pub struct DocumentStore {
    inner: imp_documents::DocumentStore<DocumentPayload>,
    origins: HashMap<DocumentId, DocumentOrigin>,
}

impl Deref for DocumentStore {
    type Target = imp_documents::DocumentStore<DocumentPayload>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for DocumentStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl DocumentStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Origin for an open document, when known (untitled tabs have none).
    pub fn origin(&self, id: DocumentId) -> Option<&DocumentOrigin> {
        self.origins.get(&id)
    }

    /// Closes a tab and drops its origin metadata.
    pub fn close(&mut self, id: DocumentId) {
        self.origins.remove(&id);
        self.inner.close(id);
    }

    /// Assigns a filesystem path after Save As and records a filesystem origin.
    pub fn set_path_and_clean(&mut self, id: DocumentId, path: PathBuf) {
        self.origins
            .insert(id, DocumentOrigin::Filesystem { path: path.clone() });
        self.inner.set_path_and_clean(id, path);
    }

    /// Clears dirty after a successful save without changing Miro origin.
    pub fn mark_clean(&mut self, id: DocumentId) {
        self.inner.mark_clean(id);
    }

    /// Opens a stacked document at `path`, or activates an existing tab for the same path.
    ///
    /// Returns the document id that is active afterward, or `None` if the tip payload
    /// cannot be stored (unsupported tip type).
    pub fn open_or_activate_stacked(
        &mut self,
        path: PathBuf,
        opened: StackedOpen,
    ) -> Option<DocumentId> {
        if let Some(id) = self.find_by_path(&path).map(|doc| doc.id) {
            self.select(id);
            return Some(id);
        }

        let tip_type = opened.type_chain.last()?.as_str();
        let payload = DocumentPayload::from_tip(tip_type, opened.payload)?;
        let id = self.inner.open_or_activate(
            path.clone(),
            opened.type_chain,
            opened.roles,
            payload,
        );
        self.origins
            .insert(id, DocumentOrigin::Filesystem { path });
        Some(id)
    }

    /// Opens a Miro-pulled stacked document, or activates an existing Miro tab.
    ///
    /// Dedupes on `(content_root, mir_record_id)` so two NodeIds in the same Mir
    /// blob share one tab. Sets the session path from the Mir record id for Save.
    pub fn open_or_activate_miro(
        &mut self,
        content_root: PathBuf,
        node_id: String,
        mir_record_id: String,
        opened: StackedOpen,
    ) -> Option<DocumentId> {
        if let Some(id) = self.find_miro_blob(&content_root, &mir_record_id) {
            self.select(id);
            return Some(id);
        }

        let tip_type = opened.type_chain.last()?.as_str();
        let payload = DocumentPayload::from_tip(tip_type, opened.payload)?;
        let authoring = authoring_path_for_mir_record(&content_root, &mir_record_id);
        let id = self.inner.open_or_activate(
            authoring,
            opened.type_chain,
            opened.roles,
            payload,
        );
        self.origins.insert(
            id,
            DocumentOrigin::Miro {
                content_root,
                node_id,
                mir_record_id,
            },
        );
        Some(id)
    }

    fn find_miro_blob(&self, content_root: &Path, mir_record_id: &str) -> Option<DocumentId> {
        self.origins.iter().find_map(|(id, origin)| match origin {
            DocumentOrigin::Miro {
                content_root: root,
                mir_record_id: rid,
                ..
            } if root == content_root && rid == mir_record_id => Some(*id),
            _ => None,
        })
    }

    /// Creates an untitled notation tab and activates it.
    pub fn create_untitled_notation(&mut self) -> DocumentId {
        let index = self.inner.take_untitled_index();
        let title = if index == 0 {
            String::from("Untitled")
        } else {
            format!("Untitled-{index}")
        };
        self.inner.insert_untitled(
            title,
            vec![
                aura_interfaces::DOC_TYPE_JSON.to_string(),
                aura_interfaces::DOC_TYPE_AURA_JSON.to_string(),
                aura_interfaces::DOC_TYPE_NOTATION.to_string(),
            ],
            vec![
                Some(aura_interfaces::ROLE_JSON.to_string()),
                None,
                Some(aura_interfaces::ROLE_NOTATION.to_string()),
            ],
            DocumentPayload::Model(ModelGraph::from_notation(Notation::empty())),
        )
    }

    /// Creates an untitled empty Imp graph tab and activates it.
    pub fn create_untitled_graph(&mut self) -> DocumentId {
        let index = self.inner.take_untitled_index();
        let title = if index == 0 {
            String::from("Untitled graph")
        } else {
            format!("Untitled graph-{index}")
        };
        self.inner.insert_untitled(
            title,
            vec![
                aura_interfaces::DOC_TYPE_JSON.to_string(),
                aura_interfaces::DOC_TYPE_AURA_JSON.to_string(),
                aura_interfaces::DOC_TYPE_IMP_GRAPH.to_string(),
            ],
            vec![
                Some(aura_interfaces::ROLE_JSON.to_string()),
                None,
                Some(aura_interfaces::ROLE_IMP_GRAPH.to_string()),
            ],
            DocumentPayload::Model(ModelGraph::from_imp_graph(Graph {
                nodes: Default::default(),
                edges: Default::default(),
            })),
        )
    }

    /// Opens `graph` at `path`, or activates an existing tab for the same path.
    ///
    /// Convenience for tests that construct an Imp graph directly.
    pub fn open_or_activate(&mut self, path: PathBuf, graph: Graph) -> DocumentId {
        let opened = StackedOpen {
            type_chain: vec![
                aura_interfaces::DOC_TYPE_JSON.to_string(),
                aura_interfaces::DOC_TYPE_AURA_JSON.to_string(),
                aura_interfaces::DOC_TYPE_IMP_GRAPH.to_string(),
            ],
            roles: vec![
                Some(aura_interfaces::ROLE_JSON.to_string()),
                None,
                Some(aura_interfaces::ROLE_IMP_GRAPH.to_string()),
            ],
            payload: Box::new(graph),
        };
        self.open_or_activate_stacked(path, opened)
            .expect("imp-graph tip is always storable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_ext_notation::NotationNote;

    #[test]
    fn notation_tip_wraps_as_nullary_external_score_node() {
        let mut notation = Notation::empty();
        notation.events.push(NotationNote {
            semitone: 60,
            start_beats: 0.0,
            duration_beats: 1.0,
            velocity: 1.0,
        });
        let payload = DocumentPayload::from_tip(
            aura_interfaces::DOC_TYPE_NOTATION,
            Box::new(notation.clone()),
        )
        .expect("notation tip");

        assert!(payload.graph().is_none());
        let embedded = payload.notation().expect("embedded notation");
        assert_eq!(embedded.events.len(), 1);

        let model = payload.model().expect("model");
        let node = model
            .graph
            .nodes
            .get(NOTATION_SCORE_NODE_ID)
            .expect("legacy score node");
        assert_eq!(node.node_type, NOTATION_SCORE_NODE_TYPE);
        assert!(node.inputs.is_empty());
        assert!(model.graph.edges.is_empty());
    }

    #[test]
    fn notation_tip_id_becomes_model_node_id() {
        let mut notation = Notation::empty();
        notation.id = Some("TIPID00000000000000000001".into());
        let model = ModelGraph::from_notation(notation);
        assert!(model.graph.nodes.contains_key("TIPID00000000000000000001"));
        assert!(!model.graph.nodes.contains_key(NOTATION_SCORE_NODE_ID));
        assert_eq!(
            model.notation().expect("notation").tip_node_id(),
            "TIPID00000000000000000001"
        );
    }

    #[test]
    fn graph_tip_loads_as_model_without_embedded() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "time".to_string(),
            Node {
                id: "time".to_string(),
                title: None,
                node_type: "time".to_string(),
                type_args: Vec::new(),
                inputs: BTreeMap::new(),
            },
        );
        let graph = Graph {
            nodes,
            edges: BTreeMap::new(),
        };
        let payload =
            DocumentPayload::from_tip(aura_interfaces::DOC_TYPE_IMP_GRAPH, Box::new(graph))
                .expect("graph tip");

        assert!(payload.notation().is_none());
        let loaded = payload.graph().expect("imp graph");
        assert!(loaded.nodes.contains_key("time"));
        assert!(payload.model().expect("model").embedded.is_empty());
    }

    #[test]
    fn untitled_notation_round_trips_embedded_edits() {
        let mut store = DocumentStore::new();
        let id = store.create_untitled_notation();
        let doc = store.get_mut(id).expect("doc");
        let notation = doc.payload.notation_mut().expect("notation");
        notation.tempo_bpm = 90.0;
        assert_eq!(doc.payload.notation().expect("notation").tempo_bpm, 90.0);
        assert!(doc.payload.graph().is_none());
    }

    #[test]
    fn title_for_path_uses_stem() {
        assert_eq!(
            imp_documents::title_for_path(std::path::Path::new("/tmp/demo.json")),
            "demo"
        );
    }

    #[test]
    fn miro_origin_dedupes_by_mir_record_id() {
        let mut store = DocumentStore::new();
        let root = PathBuf::from("/content");
        let opened = StackedOpen {
            type_chain: vec![
                aura_interfaces::DOC_TYPE_JSON.to_string(),
                aura_interfaces::DOC_TYPE_AURA_JSON.to_string(),
                aura_interfaces::DOC_TYPE_IMP_GRAPH.to_string(),
            ],
            roles: vec![
                Some(aura_interfaces::ROLE_JSON.to_string()),
                None,
                Some(aura_interfaces::ROLE_IMP_GRAPH.to_string()),
            ],
            payload: Box::new(Graph {
                nodes: BTreeMap::new(),
                edges: BTreeMap::new(),
            }),
        };
        let id1 = store
            .open_or_activate_miro(
                root.clone(),
                "NODEA".into(),
                "a.json".into(),
                opened,
            )
            .expect("open");
        let opened2 = StackedOpen {
            type_chain: vec![
                aura_interfaces::DOC_TYPE_JSON.to_string(),
                aura_interfaces::DOC_TYPE_AURA_JSON.to_string(),
                aura_interfaces::DOC_TYPE_IMP_GRAPH.to_string(),
            ],
            roles: vec![
                Some(aura_interfaces::ROLE_JSON.to_string()),
                None,
                Some(aura_interfaces::ROLE_IMP_GRAPH.to_string()),
            ],
            payload: Box::new(Graph {
                nodes: BTreeMap::new(),
                edges: BTreeMap::new(),
            }),
        };
        // Different NodeId, same Mir blob → same tab.
        let id2 = store
            .open_or_activate_miro(
                root.clone(),
                "NODEB".into(),
                "a.json".into(),
                opened2,
            )
            .expect("reopen");
        assert_eq!(id1, id2);
        assert_eq!(store.documents().len(), 1);
        assert_eq!(
            store.origin(id1),
            Some(&DocumentOrigin::Miro {
                content_root: root,
                node_id: "NODEA".into(),
                mir_record_id: "a.json".into(),
            })
        );
    }
}
