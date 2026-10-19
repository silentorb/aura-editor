//! Builtin stackable document types: json → aura-json → imp-graph / imp-graph-library.

use std::any::Any;
use std::path::Path;
use std::sync::Arc;

use aura_interfaces::{
    AuraJsonEnvelope, DocumentOpenError, DocumentType, DocumentTypeHost, OpenSource,
    AURA_JSON_TYPE_PROPERTY, DOC_TYPE_AURA_JSON, DOC_TYPE_IMP_GRAPH, DOC_TYPE_IMP_GRAPH_LIBRARY,
    DOC_TYPE_JSON, ROLE_IMP_GRAPH, ROLE_JSON,
};
use imp_core_types::Graph;
use serde_json::Value;

/// Registers builtin document-type implementations.
pub fn register_document_types(host: &mut impl DocumentTypeHost) {
    host.register_document_type(DOC_TYPE_JSON, Arc::new(JsonDocumentType));
    host.register_document_type(DOC_TYPE_AURA_JSON, Arc::new(AuraJsonDocumentType));
    host.register_document_type(DOC_TYPE_IMP_GRAPH, Arc::new(ImpGraphDocumentType));
    host.register_document_type(
        DOC_TYPE_IMP_GRAPH_LIBRARY,
        Arc::new(ImpGraphLibraryDocumentType),
    );
}

struct JsonDocumentType;

impl DocumentType for JsonDocumentType {
    fn parent_id(&self) -> Option<&str> {
        None
    }

    fn role(&self) -> Option<&str> {
        Some(ROLE_JSON)
    }

    fn matches_root(&self, source: &OpenSource<'_>) -> bool {
        let Some(text) = source.text() else {
            return false;
        };
        serde_json::from_str::<Value>(text).is_ok()
    }

    fn matches_child(&self, _parent_payload: &dyn Any) -> bool {
        false
    }

    fn open_root(
        &self,
        source: &OpenSource<'_>,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        let text = source.text().ok_or_else(|| DocumentOpenError::Invalid {
            path: source.path.to_path_buf(),
            reason: String::from("file is not valid UTF-8"),
        })?;
        let value: Value =
            serde_json::from_str(text).map_err(|err| DocumentOpenError::Json {
                path: source.path.to_path_buf(),
                message: err.to_string(),
            })?;
        Ok(Box::new(value))
    }

    fn open_child(
        &self,
        path: &Path,
        _parent_payload: &dyn Any,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        Err(DocumentOpenError::Invalid {
            path: path.to_path_buf(),
            reason: String::from("json is a root document type"),
        })
    }
}

struct AuraJsonDocumentType;

impl DocumentType for AuraJsonDocumentType {
    fn parent_id(&self) -> Option<&str> {
        Some(DOC_TYPE_JSON)
    }

    fn role(&self) -> Option<&str> {
        None
    }

    fn matches_root(&self, _source: &OpenSource<'_>) -> bool {
        false
    }

    fn matches_child(&self, parent_payload: &dyn Any) -> bool {
        let Some(value) = parent_payload.downcast_ref::<Value>() else {
            return false;
        };
        aura_type_id(value).is_some()
    }

    fn open_root(
        &self,
        source: &OpenSource<'_>,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        Err(DocumentOpenError::Invalid {
            path: source.path.to_path_buf(),
            reason: String::from("aura-json requires a json parent payload"),
        })
    }

    fn open_child(
        &self,
        path: &Path,
        parent_payload: &dyn Any,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        let value = parent_payload
            .downcast_ref::<Value>()
            .ok_or_else(|| DocumentOpenError::Invalid {
                path: path.to_path_buf(),
                reason: String::from("aura-json parent payload is not JSON"),
            })?;
        let type_id = aura_type_id(value).ok_or_else(|| DocumentOpenError::Invalid {
            path: path.to_path_buf(),
            reason: String::from("Aura JSON requires a root string \"type\" property"),
        })?;
        Ok(Box::new(AuraJsonEnvelope {
            type_id,
            value: value.clone(),
        }))
    }
}

struct ImpGraphDocumentType;

impl DocumentType for ImpGraphDocumentType {
    fn parent_id(&self) -> Option<&str> {
        Some(DOC_TYPE_AURA_JSON)
    }

    fn role(&self) -> Option<&str> {
        Some(ROLE_IMP_GRAPH)
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
            reason: String::from("imp-graph requires an aura-json parent payload"),
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
                reason: String::from("imp-graph parent payload is not an Aura JSON envelope"),
            })?;
        let graph: Graph = serde_json::from_value(envelope.value.clone()).map_err(|err| {
            DocumentOpenError::Invalid {
                path: path.to_path_buf(),
                reason: err.to_string(),
            }
        })?;
        Ok(Box::new(graph))
    }
}

struct ImpGraphLibraryDocumentType;

impl DocumentType for ImpGraphLibraryDocumentType {
    fn parent_id(&self) -> Option<&str> {
        Some(DOC_TYPE_AURA_JSON)
    }

    fn role(&self) -> Option<&str> {
        None
    }

    fn matches_root(&self, _source: &OpenSource<'_>) -> bool {
        false
    }

    fn matches_child(&self, _parent_payload: &dyn Any) -> bool {
        false
    }

    fn open_root(
        &self,
        source: &OpenSource<'_>,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        Err(DocumentOpenError::Invalid {
            path: source.path.to_path_buf(),
            reason: String::from("imp-graph-library requires an aura-json parent payload"),
        })
    }

    fn open_child(
        &self,
        path: &Path,
        parent_payload: &dyn Any,
    ) -> Result<Box<dyn Any + Send + Sync>, DocumentOpenError> {
        let _envelope = parent_payload
            .downcast_ref::<AuraJsonEnvelope>()
            .ok_or_else(|| DocumentOpenError::Invalid {
                path: path.to_path_buf(),
                reason: String::from(
                    "imp-graph-library parent payload is not an Aura JSON envelope",
                ),
            })?;
        Err(DocumentOpenError::GraphLibraryNotSupported {
            path: path.to_path_buf(),
        })
    }
}

fn aura_type_id(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    obj.get(AURA_JSON_TYPE_PROPERTY)?
        .as_str()
        .map(str::to_owned)
}
