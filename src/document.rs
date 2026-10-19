//! Open documents and tab ordering for the multi-document shell.

use std::any::Any;
use std::path::{Path, PathBuf};

use imp_core_types::Graph;
use serde_json::Value;

use crate::extensions::StackedOpen;

/// Stable identity for an open document within the editor session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
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
            Self::Json(_) => None,
        }
    }
}

/// An open document held in the editor session.
#[derive(Debug, Clone)]
pub struct SessionDocument {
    /// Session-local identity.
    pub id: DocumentId,
    /// Absolute or user-supplied path used to open the file.
    pub path: PathBuf,
    /// Tab title (file stem when available).
    pub title: String,
    /// Document-type ids from root to tip.
    pub type_chain: Vec<String>,
    /// Parallel optional view roles for each chain entry.
    pub roles: Vec<Option<String>>,
    /// Tip payload.
    pub payload: DocumentPayload,
}

/// Backward-compatible alias used by older call sites.
pub type GraphDocument = SessionDocument;

/// Ordered collection of open documents plus the active tab.
#[derive(Debug, Clone, Default)]
pub struct DocumentStore {
    documents: Vec<SessionDocument>,
    active: Option<DocumentId>,
    next_id: u64,
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

    /// Finds an open document by filesystem path.
    pub fn find_by_path(&self, path: &Path) -> Option<&SessionDocument> {
        self.documents.iter().find(|doc| doc.path == path)
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
        if let Some(existing) = self.documents.iter().find(|doc| doc.path == path) {
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
            path,
            title,
            type_chain: opened.type_chain,
            roles: opened.roles,
            payload,
        });
        self.active = Some(id);
        Some(id)
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
