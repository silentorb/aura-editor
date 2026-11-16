//! Builtin Aura Editor extension: document commands, document types, and the graph-dag view.

mod document_types;

use aura_interfaces::{
    CommandDescriptor, CommandHost, DocumentTypeHost, UserStateDescriptor, UserStateHost,
    ViewDescriptor, ViewHost, CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_NEW_GRAPH,
    CMD_DOCUMENT_OPEN, CMD_DOCUMENT_OPEN_FILE, CMD_DOCUMENT_OPEN_WORKSPACE, CMD_GRAPH_ADD_NODE,
    CMD_GRAPH_DELETE_SELECTION, CMD_PLAY_START, CMD_PLAY_STOP, CMD_PLAY_TOGGLE, ROLE_IMP_GRAPH,
    USER_STATE_GRAPH_DAG_UI, USER_STATE_OPEN_DOCUMENTS, USER_STATE_SHELL_LAYOUT, VIEW_GRAPH_DAG,
};

use crate::document_types::register_document_types;

/// Registers builtin command, view, document-type, and user-state implementations with the host.
pub fn register(host: &mut (impl CommandHost + ViewHost + DocumentTypeHost + UserStateHost)) {
    host.register_command(
        CMD_DOCUMENT_OPEN,
        CommandDescriptor::with_label("Open by NodeId"),
    );
    host.register_command(
        CMD_DOCUMENT_OPEN_WORKSPACE,
        CommandDescriptor::with_label("Open workspace"),
    );
    host.register_command(
        CMD_DOCUMENT_OPEN_FILE,
        CommandDescriptor::with_label("Open file"),
    );
    host.register_command(
        CMD_DOCUMENT_CLOSE_ACTIVE,
        CommandDescriptor::with_label("Close active document"),
    );
    host.register_command(
        CMD_DOCUMENT_NEW_GRAPH,
        CommandDescriptor::with_label("New graph"),
    );
    host.register_command(CMD_PLAY_START, CommandDescriptor::with_label("Start playback"));
    host.register_command(CMD_PLAY_STOP, CommandDescriptor::with_label("Stop playback"));
    host.register_command(
        CMD_PLAY_TOGGLE,
        CommandDescriptor::with_label("Toggle playback"),
    );
    host.register_command(
        CMD_GRAPH_ADD_NODE,
        CommandDescriptor::with_label("Add graph node"),
    );
    host.register_command(
        CMD_GRAPH_DELETE_SELECTION,
        CommandDescriptor::with_label("Delete graph selection"),
    );
    host.register_view(
        VIEW_GRAPH_DAG,
        ViewDescriptor::with_roles([ROLE_IMP_GRAPH]),
    );
    host.register_user_state(
        USER_STATE_OPEN_DOCUMENTS,
        UserStateDescriptor::with_label("Open documents"),
    );
    host.register_user_state(
        USER_STATE_GRAPH_DAG_UI,
        UserStateDescriptor::with_label("Graph DAG UI"),
    );
    host.register_user_state(
        USER_STATE_SHELL_LAYOUT,
        UserStateDescriptor::with_label("Shell layout"),
    );
    register_document_types(host);
}
