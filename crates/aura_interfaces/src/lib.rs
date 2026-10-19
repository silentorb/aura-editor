//! Extension contracts and `extensions.json` types for Aura Editor.
//!
//! Extension packages depend on this crate only (not on `aura_editor`).

mod command;
mod config;
mod document_type;
mod view;

pub use command::{CommandDescriptor, CommandHost};
pub use config::{
    parse_extensions_json, ComponentConfig, ComponentKind, ExtensionConfig, ExtensionsConfig,
    ExtensionsError, SUPPORTED_VERSION,
};
pub use document_type::{
    AuraJsonEnvelope, DocumentOpenError, DocumentType, DocumentTypeHost, OpenSource,
};
pub use view::{ViewDescriptor, ViewHost};

/// Root property on Aura JSON objects naming the child document-type id.
pub const AURA_JSON_TYPE_PROPERTY: &str = "type";

/// Document role used for Imp single-graph documents.
pub const ROLE_IMP_GRAPH: &str = "imp-graph";

/// Document role for generic JSON documents (fallback view hook).
pub const ROLE_JSON: &str = "json";

/// Builtin extension package id.
pub const EXT_AURA_BUILTIN: &str = "aura-builtin";

/// Builtin command implementation / component ids.
pub const CMD_DOCUMENT_OPEN: &str = "document.open";
pub const CMD_DOCUMENT_CLOSE_ACTIVE: &str = "document.close_active";

/// Builtin graph DAG view implementation / component id.
pub const VIEW_GRAPH_DAG: &str = "graph-dag";

/// Builtin stackable document-type ids.
pub const DOC_TYPE_JSON: &str = "json";
pub const DOC_TYPE_AURA_JSON: &str = "aura-json";
pub const DOC_TYPE_IMP_GRAPH: &str = "imp-graph";
pub const DOC_TYPE_IMP_GRAPH_LIBRARY: &str = "imp-graph-library";
