//! Extension contracts and `extensions.json` types for Aura Editor.
//!
//! Extension packages depend on this crate only (not on `aura_editor`).

mod command;
mod config;
mod document_type;
mod user_state;
mod view;

pub use command::{CommandDescriptor, CommandHost};
pub use config::{
    parse_extensions_json, ComponentConfig, ComponentKind, ExtensionConfig, ExtensionsConfig,
    ExtensionsError, SUPPORTED_VERSION,
};
pub use document_type::{
    AuraJsonEnvelope, DocumentOpenError, DocumentType, DocumentTypeHost, OpenSource,
};
pub use user_state::{UserStateDescriptor, UserStateHost};
pub use view::{ViewDescriptor, ViewHost};

/// Root property on Aura JSON objects naming the child document-type id.
pub const AURA_JSON_TYPE_PROPERTY: &str = "type";

/// Document role used for Imp single-graph documents.
pub const ROLE_IMP_GRAPH: &str = "imp-graph";

/// Document role for notation documents.
pub const ROLE_NOTATION: &str = "notation";

/// Document role for generic JSON documents (fallback view hook).
pub const ROLE_JSON: &str = "json";

/// Builtin extension package id.
pub const EXT_AURA_BUILTIN: &str = "aura-builtin";

/// Notation extension package id.
pub const EXT_AURA_NOTATION: &str = "aura-notation";

/// Builtin command implementation / component ids.
pub const CMD_DOCUMENT_OPEN: &str = "document.open";
pub const CMD_DOCUMENT_CLOSE_ACTIVE: &str = "document.close_active";
pub const CMD_DOCUMENT_NEW_GRAPH: &str = "document.new.graph";
pub const CMD_PLAY_START: &str = "play.start";
pub const CMD_PLAY_STOP: &str = "play.stop";
pub const CMD_PLAY_TOGGLE: &str = "play.toggle";
pub const CMD_GRAPH_TOOL_SELECT: &str = "graph.tool.select";
pub const CMD_GRAPH_TOOL_ADD: &str = "graph.tool.add";
pub const CMD_GRAPH_TOOL_CONNECT: &str = "graph.tool.connect";
pub const CMD_GRAPH_TOOL_ERASE: &str = "graph.tool.erase";
pub const CMD_GRAPH_DELETE_SELECTION: &str = "graph.delete.selection";

/// Notation extension command ids.
pub const CMD_DOCUMENT_NEW_NOTATION: &str = "document.new.notation";
pub const CMD_DOCUMENT_SAVE: &str = "document.save";
pub const CMD_NOTATION_TOOL_DRAW: &str = "notation.tool.draw";
pub const CMD_NOTATION_TOOL_SELECT: &str = "notation.tool.select";
pub const CMD_NOTATION_TOOL_ERASE: &str = "notation.tool.erase";
pub const CMD_NOTATION_SNAP_TOGGLE: &str = "notation.snap.toggle";

/// Builtin graph DAG view implementation / component id.
pub const VIEW_GRAPH_DAG: &str = "graph-dag";

/// Piano roll view implementation / component id.
pub const VIEW_PIANO_ROLL: &str = "piano-roll";

/// Builtin stackable document-type ids.
pub const DOC_TYPE_JSON: &str = "json";
pub const DOC_TYPE_AURA_JSON: &str = "aura-json";
pub const DOC_TYPE_IMP_GRAPH: &str = "imp-graph";
pub const DOC_TYPE_IMP_GRAPH_LIBRARY: &str = "imp-graph-library";

/// Notation document-type id.
pub const DOC_TYPE_NOTATION: &str = "notation";

/// Builtin user-state entry for open document tabs.
pub const USER_STATE_OPEN_DOCUMENTS: &str = "open-documents";

/// Piano roll UI prefs user-state entry.
pub const USER_STATE_PIANO_ROLL_UI: &str = "piano-roll-ui";

/// Graph DAG UI prefs user-state entry.
pub const USER_STATE_GRAPH_DAG_UI: &str = "graph-dag-ui";

/// Shell pane-grid layout user-state entry.
pub const USER_STATE_SHELL_LAYOUT: &str = "shell-layout";

/// Domain bucket id for open-documents user state.
pub const USER_STATE_BUCKET_DOCUMENTS: &str = "documents";

/// Domain bucket id for builtin graph UI user state.
pub const USER_STATE_BUCKET_BUILTIN: &str = "aura-builtin";

/// Domain bucket id for notation UI user state.
pub const USER_STATE_BUCKET_NOTATION: &str = "aura-notation";

/// Domain bucket id for shell pane-grid layout user state.
pub const USER_STATE_BUCKET_SHELL: &str = "shell";

/// Config param key selecting the user-state bucket file id.
pub const USER_STATE_PARAM_BUCKET: &str = "bucket";
