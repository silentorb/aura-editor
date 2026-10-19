//! Stackable document-type registration contracts.

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;

/// Raw file bytes and path presented to root document types.
#[derive(Debug, Clone, Copy)]
pub struct OpenSource<'a> {
    /// Filesystem path being opened.
    pub path: &'a Path,
    /// File contents.
    pub bytes: &'a [u8],
}

impl<'a> OpenSource<'a> {
    /// UTF-8 view of [`Self::bytes`], or `None` if not valid UTF-8.
    pub fn text(&self) -> Option<&'a str> {
        std::str::from_utf8(self.bytes).ok()
    }
}

/// Parsed Aura JSON envelope: root `type` plus the full JSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct AuraJsonEnvelope {
    /// Value of the root `type` property (document-type id).
    pub type_id: String,
    /// Full JSON object (including `type`).
    pub value: Value,
}

/// Failure while matching or opening through document types.
///
/// Causes are stored as text so the type stays `Clone` for Iced messages.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DocumentOpenError {
    /// Could not read the file.
    #[error("could not read file {}: {message}", path.display())]
    Io {
        /// Path that failed to open or read.
        path: PathBuf,
        /// Underlying I/O error message.
        message: String,
    },
    /// File contents were not valid JSON where JSON was required.
    #[error("invalid JSON in {}: {message}", path.display())]
    Json {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying JSON error message.
        message: String,
    },
    /// Payload was not valid for the document type.
    #[error("invalid document {}: {reason}", path.display())]
    Invalid {
        /// Path of the invalid file.
        path: PathBuf,
        /// Why the value is not usable.
        reason: String,
    },
    /// Imp GraphLibrary catalogs are not opened as session documents yet.
    #[error(
        "graph library not supported yet: {} (open a single-graph Aura JSON file with type \"imp-graph\")",
        path.display()
    )]
    GraphLibraryNotSupported {
        /// Path of the GraphLibrary file.
        path: PathBuf,
    },
    /// Aura JSON `type` did not match an enabled child document type.
    #[error("unknown Aura document type `{type_id}` in {}", path.display())]
    UnknownAuraType {
        /// Path being opened.
        path: PathBuf,
        /// Root `type` value that was not registered/enabled.
        type_id: String,
    },
    /// No root document type matched the file.
    #[error("unrecognized document: {}", path.display())]
    Unrecognized {
        /// Path that no document type claimed.
        path: PathBuf,
    },
}

impl DocumentOpenError {
    /// Short banner text suitable for the editor status line.
    pub fn status_message(&self) -> String {
        self.to_string()
    }
}

/// Stackable document-type implementation offered by an extension.
pub trait DocumentType: Send + Sync {
    /// Parent document-type id, or `None` for a root type.
    fn parent_id(&self) -> Option<&str>;

    /// View role for this type when chosen as tip or fallback; `None` if never viewed alone.
    fn role(&self) -> Option<&str>;

    /// Whether this root type claims `source` (ignored when [`Self::parent_id`] is `Some`).
    fn matches_root(&self, source: &OpenSource<'_>) -> bool;

    /// Whether this child type claims `parent_payload` (ignored under Aura JSON id dispatch).
    fn matches_child(&self, parent_payload: &dyn Any) -> bool;

    /// Load a root payload from raw bytes.
    fn open_root(
        &self,
        source: &OpenSource<'_>,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError>;

    /// Refine a parent payload into this type's payload.
    fn open_child(
        &self,
        path: &Path,
        parent_payload: &dyn Any,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError>;
}

/// Host surface that accepts document-type implementations from extensions.
pub trait DocumentTypeHost {
    /// Registers a document-type implementation under `implementation_id`.
    fn register_document_type(
        &mut self,
        implementation_id: &str,
        document_type: Arc<dyn DocumentType>,
    );
}
