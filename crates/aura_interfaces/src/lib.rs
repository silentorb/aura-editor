//! Extension contracts and `extensions.json` types for Aura Editor.
//!
//! Extension packages depend on this crate only (not on `aura_editor`).

mod command;
mod config;
mod view;

pub use command::{CommandDescriptor, CommandHost};
pub use config::{
    parse_extensions_json, ComponentConfig, ComponentKind, ExtensionConfig, ExtensionsConfig,
    ExtensionsError, SUPPORTED_VERSION,
};
pub use view::{ViewDescriptor, ViewHost};

/// Document role used for Imp single-graph documents in the v1 host.
pub const ROLE_IMP_GRAPH: &str = "imp-graph";

/// Builtin extension package id.
pub const EXT_AURA_BUILTIN: &str = "aura-builtin";

/// Builtin command implementation / component ids.
pub const CMD_DOCUMENT_OPEN: &str = "document.open";
pub const CMD_DOCUMENT_CLOSE_ACTIVE: &str = "document.close_active";

/// Builtin graph DAG view implementation / component id.
pub const VIEW_GRAPH_DAG: &str = "graph-dag";
