//! Builtin Aura Editor extension: document commands and the graph-dag view.

use aura_interfaces::{
    CommandDescriptor, CommandHost, ViewDescriptor, ViewHost, CMD_DOCUMENT_CLOSE_ACTIVE,
    CMD_DOCUMENT_OPEN, ROLE_IMP_GRAPH, VIEW_GRAPH_DAG,
};

/// Registers builtin command and view implementations with the host.
pub fn register(host: &mut (impl CommandHost + ViewHost)) {
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
}
