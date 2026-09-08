//! Open graph documents and tab ordering for the multi-document shell.

use std::path::{Path, PathBuf};

use imp_core_types::Graph;

/// Stable identity for an open document within the editor session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
    /// Numeric value for tests and diagnostics.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// An open Imp graph document held in the editor.
#[derive(Debug, Clone)]
pub struct GraphDocument {
    /// Session-local identity.
    pub id: DocumentId,
    /// Absolute or user-supplied path used to open the file.
    pub path: PathBuf,
    /// Tab title (file stem when available).
    pub title: String,
    /// Parsed Imp graph.
    pub graph: Graph,
}

/// Ordered collection of open documents plus the active tab.
#[derive(Debug, Clone, Default)]
pub struct DocumentStore {
    documents: Vec<GraphDocument>,
    active: Option<DocumentId>,
    next_id: u64,
}

impl DocumentStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open documents in tab order.
    pub fn documents(&self) -> &[GraphDocument] {
        &self.documents
    }

    /// Currently active document id, if any.
    pub fn active_id(&self) -> Option<DocumentId> {
        self.active
    }

    /// Active document, if any.
    pub fn active_document(&self) -> Option<&GraphDocument> {
        let id = self.active?;
        self.documents.iter().find(|doc| doc.id == id)
    }

    /// Finds an open document by filesystem path.
    pub fn find_by_path(&self, path: &Path) -> Option<&GraphDocument> {
        self.documents.iter().find(|doc| doc.path == path)
    }

    /// Opens `graph` at `path`, or activates an existing tab for the same path.
    ///
    /// Returns the document id that is active afterward.
    pub fn open_or_activate(&mut self, path: PathBuf, graph: Graph) -> DocumentId {
        if let Some(existing) = self.documents.iter().find(|doc| doc.path == path) {
            let id = existing.id;
            self.active = Some(id);
            return id;
        }

        let id = DocumentId(self.next_id);
        self.next_id += 1;
        let title = title_for_path(&path);
        self.documents.push(GraphDocument {
            id,
            path,
            title,
            graph,
        });
        self.active = Some(id);
        id
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
