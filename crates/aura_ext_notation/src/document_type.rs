//! `notation` document type under the Aura JSON stack.

use std::any::Any;
use std::path::Path;
use std::sync::Arc;

use aura_interfaces::{
    AuraJsonEnvelope, DocumentOpenError, DocumentType, DocumentTypeHost, OpenSource,
    DOC_TYPE_AURA_JSON, DOC_TYPE_NOTATION, ROLE_NOTATION,
};

use crate::notation::Notation;

/// Registers the notation document-type implementation.
pub fn register_document_type(host: &mut impl DocumentTypeHost) {
    host.register_document_type(DOC_TYPE_NOTATION, Arc::new(NotationDocumentType));
}

struct NotationDocumentType;

impl DocumentType for NotationDocumentType {
    fn parent_id(&self) -> Option<&str> {
        Some(DOC_TYPE_AURA_JSON)
    }

    fn role(&self) -> Option<&str> {
        Some(ROLE_NOTATION)
    }

    fn matches_root(&self, _source: &OpenSource<'_>) -> bool {
        false
    }

    fn matches_child(&self, _parent_payload: &dyn Any) -> bool {
        // Selected by Aura JSON `type` id dispatch, not linear match.
        false
    }

    fn open_root(
        &self,
        source: &OpenSource<'_>,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        Err(DocumentOpenError::Invalid {
            path: source.path.to_path_buf(),
            reason: String::from("notation requires an aura-json parent payload"),
        })
    }

    fn open_child(
        &self,
        path: &Path,
        parent_payload: &dyn Any,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        let envelope = parent_payload
            .downcast_ref::<AuraJsonEnvelope>()
            .ok_or_else(|| DocumentOpenError::Invalid {
                path: path.to_path_buf(),
                reason: String::from("notation parent payload is not an Aura JSON envelope"),
            })?;
        let notation = Notation::from_value(envelope.value.clone()).map_err(|reason| {
            DocumentOpenError::Invalid {
                path: path.to_path_buf(),
                reason,
            }
        })?;
        Ok(Box::new(notation))
    }
}
