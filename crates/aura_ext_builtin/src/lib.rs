//! Builtin Aura Editor extension: document commands, document types, and the graph-dag view.

mod document_types;

use aura_interfaces::{
    CommandDescriptor, CommandHost, DocumentTypeHost, UserStateDescriptor, UserStateHost,
    ViewDescriptor, ViewHost, CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_OPEN, CMD_PLAY_START,
    CMD_PLAY_STOP, CMD_PLAY_TOGGLE, ROLE_IMP_GRAPH, USER_STATE_OPEN_DOCUMENTS, VIEW_GRAPH_DAG,
};

use crate::document_types::register_document_types;

/// Registers builtin command, view, document-type, and user-state implementations with the host.
pub fn register(host: &mut (impl CommandHost + ViewHost + DocumentTypeHost + UserStateHost)) {
    host.register_command(
        CMD_DOCUMENT_OPEN,
        CommandDescriptor::with_label("Open document"),
    );
    host.register_command(
        CMD_DOCUMENT_CLOSE_ACTIVE,
        CommandDescriptor::with_label("Close active document"),
    );
    host.register_command(CMD_PLAY_START, CommandDescriptor::with_label("Start playback"));
    host.register_command(CMD_PLAY_STOP, CommandDescriptor::with_label("Stop playback"));
    host.register_command(
        CMD_PLAY_TOGGLE,
        CommandDescriptor::with_label("Toggle playback"),
    );
    host.register_view(
        VIEW_GRAPH_DAG,
        ViewDescriptor::with_roles([ROLE_IMP_GRAPH]),
    );
    host.register_user_state(
        USER_STATE_OPEN_DOCUMENTS,
        UserStateDescriptor::with_label("Open documents"),
    );
    register_document_types(host);
}
