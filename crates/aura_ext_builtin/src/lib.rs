//! Builtin Aura Editor extension: document commands, document types, and the graph-dag view.

mod document_types;

use aura_interfaces::{
    CommandDescriptor, CommandHost, DocumentTypeHost, ViewDescriptor, ViewHost,
    CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_OPEN, ROLE_IMP_GRAPH, VIEW_GRAPH_DAG,
};

use crate::document_types::register_document_types;

/// Registers builtin command, view, and document-type implementations with the host.
pub fn register(host: &mut (impl CommandHost + ViewHost + DocumentTypeHost)) {
    host.register_command(
        CMD_DOCUMENT_OPEN,
        CommandDescriptor::with_label("Open document"),
    );
    host.register_command(
        CMD_DOCUMENT_CLOSE_ACTIVE,
        CommandDescriptor::with_label("Close active document"),
    );
    host.register_view(
        VIEW_GRAPH_DAG,
        ViewDescriptor::with_roles([ROLE_IMP_GRAPH]),
    );
    register_document_types(host);
}
