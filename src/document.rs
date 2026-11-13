//! Open documents and Aura tip payloads for the multi-document shell.
//!
//! Session tab/path/dirty bookkeeping lives in Imp Lab’s [`imp_documents`].
//! On-disk Aura JSON formats are unchanged. In memory, editable tips are always a
//! [`ModelGraph`]: Imp graphs load as their nodes/edges; notation documents load as a
//! single nullary `external_score` node with Aura-owned embedded notation data.

use std::any::Any;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use aura_ext_notation::Notation;
use imp_core_types::{Graph, Node, NodeId};
use serde_json::Value;

use crate::extensions::StackedOpen;

/// Node id used for the nullary score node that backs a notation document tip.
pub const NOTATION_SCORE_NODE_ID: &str = "score";

/// Imp node type for notation-document nullary literals (host-provided score).
pub const NOTATION_SCORE_NODE_TYPE: &str = "external_score";

pub use imp_documents::DocumentId;

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
    pub fn from_notation(notation: Notation) -> Self {
        let node_id = NOTATION_SCORE_NODE_ID.to_string();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            node_id.clone(),
            Node {
                id: node_id.clone(),
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
        match self.embedded.get(NOTATION_SCORE_NODE_ID)? {
            EmbeddedPayload::Notation(notation) => Some(notation),
        }
    }

    /// Mutable embedded notation on the notation-document nullary node, if present.
    pub fn notation_mut(&mut self) -> Option<&mut Notation> {
        match self.embedded.get_mut(NOTATION_SCORE_NODE_ID)? {
            EmbeddedPayload::Notation(notation) => Some(notation),
        }
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
        Some(self.inner.open_or_activate(
            path,
            opened.type_chain,
            opened.roles,
            payload,
        ))
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
            .expect("score node");
        assert_eq!(node.node_type, NOTATION_SCORE_NODE_TYPE);
        assert!(node.inputs.is_empty());
        assert!(model.graph.edges.is_empty());
    }

    #[test]
    fn graph_tip_loads_as_model_without_embedded() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "time".to_string(),
            Node {
                id: "time".to_string(),
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
}
