//! Open documents and tab ordering for the multi-document shell.

use std::any::Any;
use std::path::{Path, PathBuf};

use aura_ext_notation::Notation;
use imp_core_types::Graph;
use serde_json::Value;

use crate::extensions::StackedOpen;

/// Stable identity for an open document within the editor session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
    /// Creates an id from a raw value (tests and diagnostics).
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Numeric value for tests and diagnostics.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Tip payload held for an open session document.
#[derive(Debug, Clone)]
pub enum DocumentPayload {
    /// Imp single-graph tip (`imp-graph`).
    Graph(Graph),
    /// Notation tip (`notation`).
    Notation(Notation),
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
                .map(|graph| Self::Graph(*graph)),
            aura_interfaces::DOC_TYPE_NOTATION => payload
                .downcast::<Notation>()
                .ok()
                .map(|notation| Self::Notation(*notation)),
            aura_interfaces::DOC_TYPE_JSON => payload
                .downcast::<Value>()
                .ok()
                .map(|value| Self::Json(*value)),
            _ => None,
        }
    }

    /// Imp graph when this tip is a graph document.
    pub fn graph(&self) -> Option<&Graph> {
        match self {
            Self::Graph(graph) => Some(graph),
            Self::Notation(_) | Self::Json(_) => None,
        }
    }

    /// Notation when this tip is a notation document.
    pub fn notation(&self) -> Option<&Notation> {
        match self {
            Self::Notation(notation) => Some(notation),
            Self::Graph(_) | Self::Json(_) => None,
        }
    }

    /// Mutable notation when this tip is a notation document.
    pub fn notation_mut(&mut self) -> Option<&mut Notation> {
        match self {
            Self::Notation(notation) => Some(notation),
            Self::Graph(_) | Self::Json(_) => None,
        }
    }

    /// Mutable Imp graph when this tip is a graph document.
    pub fn graph_mut(&mut self) -> Option<&mut Graph> {
        match self {
            Self::Graph(graph) => Some(graph),
            Self::Notation(_) | Self::Json(_) => None,
        }
    }
}

/// An open document held in the editor session.
#[derive(Debug, Clone)]
pub struct SessionDocument {
    /// Session-local identity.
    pub id: DocumentId,
    /// Absolute or user-supplied path when saved/opened; `None` for untitled.
    pub path: Option<PathBuf>,
    /// Tab title (file stem when available, or Untitled…).
    pub title: String,
    /// Document-type ids from root to tip.
    pub type_chain: Vec<String>,
    /// Parallel optional view roles for each chain entry.
    pub roles: Vec<Option<String>>,
    /// Tip payload.
    pub payload: DocumentPayload,
    /// Whether the in-memory tip differs from the last saved/opened bytes.
    pub dirty: bool,
}

/// Backward-compatible alias used by older call sites.
pub type GraphDocument = SessionDocument;

/// Ordered collection of open documents plus the active tab.
#[derive(Debug, Clone, Default)]
pub struct DocumentStore {
    documents: Vec<SessionDocument>,
    active: Option<DocumentId>,
    next_id: u64,
    next_untitled: u32,
}

impl DocumentStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open documents in tab order.
    pub fn documents(&self) -> &[SessionDocument] {
        &self.documents
    }

    /// Currently active document id, if any.
    pub fn active_id(&self) -> Option<DocumentId> {
        self.active
    }

    /// Active document, if any.
    pub fn active_document(&self) -> Option<&SessionDocument> {
        let id = self.active?;
        self.documents.iter().find(|doc| doc.id == id)
    }

    /// Mutable active document, if any.
    pub fn active_document_mut(&mut self) -> Option<&mut SessionDocument> {
        let id = self.active?;
        self.documents.iter_mut().find(|doc| doc.id == id)
    }

    /// Finds an open document by filesystem path.
    pub fn find_by_path(&self, path: &Path) -> Option<&SessionDocument> {
        self.documents
            .iter()
            .find(|doc| doc.path.as_deref() == Some(path))
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
        if let Some(existing) = self
            .documents
            .iter()
            .find(|doc| doc.path.as_deref() == Some(path.as_path()))
        {
            let id = existing.id;
            self.active = Some(id);
            return Some(id);
        }

        let tip_type = opened.type_chain.last()?.as_str();
        let payload = DocumentPayload::from_tip(tip_type, opened.payload)?;
        let id = DocumentId(self.next_id);
        self.next_id += 1;
        let title = title_for_path(&path);
        self.documents.push(SessionDocument {
            id,
            path: Some(path),
            title,
            type_chain: opened.type_chain,
            roles: opened.roles,
            payload,
            dirty: false,
        });
        self.active = Some(id);
        Some(id)
    }

    /// Creates an untitled notation tab and activates it.
    pub fn create_untitled_notation(&mut self) -> DocumentId {
        let id = DocumentId(self.next_id);
        self.next_id += 1;
        let index = self.next_untitled;
        self.next_untitled += 1;
        let title = if index == 0 {
            String::from("Untitled")
        } else {
            format!("Untitled-{index}")
        };
        self.documents.push(SessionDocument {
            id,
            path: None,
            title,
            type_chain: vec![
                aura_interfaces::DOC_TYPE_JSON.to_string(),
                aura_interfaces::DOC_TYPE_AURA_JSON.to_string(),
                aura_interfaces::DOC_TYPE_NOTATION.to_string(),
            ],
            roles: vec![
                Some(aura_interfaces::ROLE_JSON.to_string()),
                None,
                Some(aura_interfaces::ROLE_NOTATION.to_string()),
            ],
            payload: DocumentPayload::Notation(Notation::empty()),
            dirty: true,
        });
        self.active = Some(id);
        id
    }

    /// Creates an untitled empty Imp graph tab and activates it.
    pub fn create_untitled_graph(&mut self) -> DocumentId {
        let id = DocumentId(self.next_id);
        self.next_id += 1;
        let index = self.next_untitled;
        self.next_untitled += 1;
        let title = if index == 0 {
            String::from("Untitled graph")
        } else {
            format!("Untitled graph-{index}")
        };
        self.documents.push(SessionDocument {
            id,
            path: None,
            title,
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
            payload: DocumentPayload::Graph(Graph {
                nodes: Default::default(),
                edges: Default::default(),
            }),
            dirty: true,
        });
        self.active = Some(id);
        id
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

    /// Selects the tab for `id` when it exists.
    pub fn select(&mut self, id: DocumentId) {
        if self.documents.iter().any(|doc| doc.id == id) {
            self.active = Some(id);
        }
    }

    /// Closes the tab for `id` and selects a neighbor when needed.
    pub fn close(&mut self, id: DocumentId) {
        let Some(index) = self.documents.iter().position(|doc| doc.id == id) else {
            return;
        };
        self.documents.remove(index);

        if self.active != Some(id) {
            return;
        }

        self.active = if self.documents.is_empty() {
            None
        } else if index < self.documents.len() {
            Some(self.documents[index].id)
        } else {
            Some(self.documents[index - 1].id)
        };
    }

    /// Mutable document by id.
    pub fn get_mut(&mut self, id: DocumentId) -> Option<&mut SessionDocument> {
        self.documents.iter_mut().find(|doc| doc.id == id)
    }

    /// Marks the document dirty after an in-place edit.
    pub fn mark_dirty(&mut self, id: DocumentId) {
        if let Some(doc) = self.get_mut(id) {
            doc.dirty = true;
        }
    }

    /// Assigns a filesystem path after Save As and clears dirty.
    pub fn set_path_and_clean(&mut self, id: DocumentId, path: PathBuf) {
        if let Some(doc) = self.get_mut(id) {
            doc.title = title_for_path(&path);
            doc.path = Some(path);
            doc.dirty = false;
        }
    }

    /// Clears the dirty flag after a successful save to an existing path.
    pub fn mark_clean(&mut self, id: DocumentId) {
        if let Some(doc) = self.get_mut(id) {
            doc.dirty = false;
        }
    }
}

fn title_for_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| path.display().to_string())
}
