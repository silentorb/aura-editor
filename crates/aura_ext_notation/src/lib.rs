//! Notation extension: document type, piano-roll view, tools, and UI prefs.

mod document_type;
mod notation;

pub use notation::{
    is_black_key, semitone_label, snap_beats, Notation, NotationNote, NotationTimeSignature,
    NotationTool, SnapResolution,
};

use aura_interfaces::{
    CommandDescriptor, CommandHost, DocumentTypeHost, UserStateDescriptor, UserStateHost,
    ViewDescriptor, ViewHost, CMD_DOCUMENT_NEW_NOTATION, CMD_DOCUMENT_SAVE,
    CMD_NOTATION_SNAP_TOGGLE, CMD_NOTATION_TOOL_DRAW, CMD_NOTATION_TOOL_ERASE,
    CMD_NOTATION_TOOL_SELECT, ROLE_NOTATION, USER_STATE_PIANO_ROLL_UI, VIEW_PIANO_ROLL,
};

use crate::document_type::register_document_type;

/// Registers notation commands, view, document type, and user-state with the host.
pub fn register(
    host: &mut (impl CommandHost + ViewHost + DocumentTypeHost + UserStateHost),
) {
    host.register_command(
        CMD_DOCUMENT_NEW_NOTATION,
        CommandDescriptor::with_label("New notation"),
    );
    host.register_command(
        CMD_DOCUMENT_SAVE,
        CommandDescriptor::with_label("Save document"),
    );
    host.register_command(
        CMD_NOTATION_TOOL_DRAW,
        CommandDescriptor::with_label("Draw tool"),
    );
    host.register_command(
        CMD_NOTATION_TOOL_SELECT,
        CommandDescriptor::with_label("Select tool"),
    );
    host.register_command(
        CMD_NOTATION_TOOL_ERASE,
        CommandDescriptor::with_label("Erase tool"),
    );
    host.register_command(
        CMD_NOTATION_SNAP_TOGGLE,
        CommandDescriptor::with_label("Toggle snap"),
    );
    host.register_view(
        VIEW_PIANO_ROLL,
        ViewDescriptor::with_roles([ROLE_NOTATION]),
    );
    host.register_user_state(
        USER_STATE_PIANO_ROLL_UI,
        UserStateDescriptor::with_label("Piano roll UI"),
    );
    register_document_type(host);
}
