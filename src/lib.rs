//! Aura Editor application library.
//!
//! Keeps Iced application logic out of `main` so unit and headless functional
//! tests can exercise state and view without opening a real window.

mod bindings;
mod command;
mod document;
mod extensions;
mod graph_edit;
mod graph_file;
mod graph_layout;
mod graph_ui;
mod graph_view;
mod logging;
mod menu_bar;
mod piano_roll;
mod piano_roll_ui;
mod playback;
mod shell_layout;
mod user_state;

pub use bindings::{user_bindings_path, Bindings, DEFAULT_BINDINGS_JSON};
pub use command::CommandId;
pub use document::{
    DocumentId, DocumentPayload, DocumentStore, GraphDocument, SessionDocument,
};
pub use extensions::{
    register_linked_extensions, ExtensionRuntime, RegistrationHost, StackedOpen, UserStateBinding,
    DEFAULT_EXTENSIONS_JSON,
};
pub use graph_edit::{
    add_edge, add_node, auto_pair_ports, graph_to_aura_json, remove_edge, remove_edges_between,
    remove_node, rename_node, set_node_type, unique_node_id, GraphEditError,
};
pub use graph_file::OpenError;
pub use graph_layout::{layout_graph, GraphLayout, LaidOutNode};
pub use graph_ui::{ConnectionMode, GraphTool, GraphUiEntry, GraphUiState};
pub use graph_view::{
    center_translation, clamp_scale, pan_by, zoom_at_point, GraphEdit, GraphView, NodePortInfo,
    PortCatalog,
};
pub use logging::init_logging;
pub use piano_roll_ui::{PianoRollUiEntry, PianoRollUiState};
pub use piano_roll::{apply_edit, piano_roll_panel, PianoRollEdit, PITCH_MAX, PITCH_MIN};
pub use shell_layout::{
    default_configuration, default_state, state_from_entry, ShellAxis, ShellLayoutEntry,
    ShellLayoutTree, ShellPanelId, SHELL_LAYOUT_ENTRY_VERSION,
};
pub use user_state::{
    write_atomic, OpenDocumentsEntry, UserDir, UserStateError, UserStateStore, BUCKET_SCHEMA_VERSION,
    DEBOUNCE, DEFAULT_USER_DIR_NAME, USER_DIR_ENV,
};

pub use aura_ext_notation::{
    semitone_label, snap_beats, Notation, NotationNote, NotationTool, SnapResolution,
};
pub use aura_interfaces::{
    CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_NEW_GRAPH, CMD_DOCUMENT_NEW_NOTATION, CMD_DOCUMENT_OPEN,
    CMD_DOCUMENT_SAVE, CMD_GRAPH_DELETE_SELECTION, CMD_GRAPH_TOOL_ADD, CMD_GRAPH_TOOL_CONNECT,
    CMD_GRAPH_TOOL_ERASE, CMD_GRAPH_TOOL_SELECT, CMD_NOTATION_SNAP_TOGGLE, CMD_NOTATION_TOOL_DRAW,
    CMD_NOTATION_TOOL_ERASE, CMD_NOTATION_TOOL_SELECT, CMD_PLAY_START, CMD_PLAY_STOP,
    CMD_PLAY_TOGGLE, DOC_TYPE_AURA_JSON, DOC_TYPE_IMP_GRAPH, DOC_TYPE_IMP_GRAPH_LIBRARY,
    DOC_TYPE_JSON, DOC_TYPE_NOTATION, ROLE_IMP_GRAPH, ROLE_JSON, ROLE_NOTATION,
    USER_STATE_BUCKET_BUILTIN, USER_STATE_BUCKET_DOCUMENTS, USER_STATE_BUCKET_NOTATION,
    USER_STATE_BUCKET_SHELL, USER_STATE_GRAPH_DAG_UI, USER_STATE_OPEN_DOCUMENTS,
    USER_STATE_PIANO_ROLL_UI, USER_STATE_SHELL_LAYOUT, VIEW_GRAPH_DAG, VIEW_PIANO_ROLL,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aura_imp::{aura_registry, get_node_type, list_node_types};
use aura_io_cpal::PlaybackEngine;
use aura_sample::SampleRate;
use iced::keyboard::{self, key::Named, Key, Modifiers};
use iced::widget::{
    button, column, container, pane_grid, pick_list, row, text, text_input, Space,
};
use iced::{time, Element, Fill, Length, Subscription, Task};
use iced_aw::menu::{Menu, MenuBar};
use iced_aw::menu_items;
use tracing::warn;

use crate::graph_ui::ConnectDraft;
use crate::graph_view::graph_canvas;
use crate::playback::{
    playback_registry, playhead_start_secs, prepare_notation_playback, prepare_session_playback,
    secs_to_beats,
};

/// Root application state for the editor shell.
#[derive(Debug)]
pub struct App {
    documents: DocumentStore,
    /// Last open/load/save error shown in the status banner.
    last_error: Option<OpenError>,
    /// Non-open status text (playback errors and notices).
    status_notice: Option<String>,
    /// Resolved input bindings (may be empty).
    bindings: Bindings,
    /// Extension command/view/document-type registries.
    extensions: ExtensionRuntime,
    /// Bucketed ephemeral user state.
    user_state: UserStateStore,
    /// Notation tool / snap prefs (persisted when enabled).
    piano_roll_ui: PianoRollUiState,
    /// Graph-dag tool / connection-mode prefs (persisted when enabled).
    graph_ui: GraphUiState,
    /// Parked playhead beats keyed by open notation document.
    playheads: HashMap<DocumentId, f64>,
    /// CPAL stream owner for the active playback graph.
    playback: PlaybackEngine,
    /// When set, auto-stop once playback position reaches this time (notation audition).
    playback_end_secs: Option<f64>,
    /// Dockable shell body panes (status / toolbar / documents / properties).
    shell_panes: pane_grid::State<ShellPanelId>,
}

/// Messages produced by the multi-document shell.
#[derive(Debug, Clone)]
pub enum Message {
    /// User chose Open… (legacy message; menus use [`Message::Command`]).
    OpenClicked,
    /// Open the document at this path (from the dialog or tests).
    OpenPath(PathBuf),
    /// Opening a path failed.
    OpenFailed(OpenError),
    /// User cancelled the native file dialog.
    OpenCancelled,
    /// Activate the tab for this document.
    SelectTab(DocumentId),
    /// Close the tab for this document.
    CloseTab(DocumentId),
    /// Uncaptured keyboard input for bindings resolution.
    KeyInput {
        key: Key,
        modifiers: Modifiers,
    },
    /// Run a bindable command (from bindings or tests).
    Command(CommandId),
    /// Debounced flush of dirty user-state buckets.
    FlushUserState {
        /// Generation token from the mutating update that scheduled this flush.
        generation: u64,
    },
    /// Piano-roll edit from the canvas.
    PianoRoll(PianoRollEdit),
    /// Snap resolution chosen from the toolbar dropdown.
    SnapResolutionSelected(SnapResolution),
    /// Save the active notation document to `path` (after Save As or known path).
    SaveToPath {
        /// Document being saved.
        document_id: DocumentId,
        /// Destination path.
        path: PathBuf,
    },
    /// Periodic tick while audio is playing (playhead redraw + end detection).
    PlaybackTick,
    /// Graph-dag canvas edit.
    Graph(GraphEdit),
    /// Node type chosen for the Add tool.
    GraphAddTypeSelected(String),
    /// Connection mode from Graph view options.
    GraphConnectionModeSelected(ConnectionMode),
    /// Rename buffer edited in the Properties pane.
    GraphRenameInput(String),
    /// Commit rename of the selected graph node.
    GraphRenameCommit,
    /// Change type of the selected graph node.
    GraphSelectedTypeChanged(String),
    /// Shell pane drag / drop.
    ShellPaneDragged(pane_grid::DragEvent),
    /// Shell pane split resized.
    ShellPaneResized(pane_grid::ResizeEvent),
}

impl From<PianoRollEdit> for Message {
    fn from(edit: PianoRollEdit) -> Self {
        Self::PianoRoll(edit)
    }
}

impl From<GraphEdit> for Message {
    fn from(edit: GraphEdit) -> Self {
        Self::Graph(edit)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::with_bindings(Bindings::empty())
    }
}

impl App {
    /// Creates the initial application state with bundled extensions and bindings.
    ///
    /// Loads user-state buckets from the resolved Aura user directory and restores
    /// open documents when the `open-documents` entry is present.
    pub fn new() -> Self {
        let extensions = ExtensionRuntime::load_bundled();
        let bindings = Bindings::load(extensions.command_ids());
        Self::with_user_dir(UserDir::resolve(), extensions, bindings)
    }

    /// Creates an app with bundled extensions and an explicit bindings table.
    ///
    /// Uses an in-memory user-state store (no disk I/O); preferred by unit tests.
    pub fn with_bindings(bindings: Bindings) -> Self {
        Self::with_extensions(ExtensionRuntime::load_bundled(), bindings)
    }

    /// Creates an app with explicit extension registries and bindings (tests).
    ///
    /// Uses an in-memory user-state store (no disk I/O).
    pub fn with_extensions(extensions: ExtensionRuntime, bindings: Bindings) -> Self {
        Self {
            documents: DocumentStore::default(),
            last_error: None,
            status_notice: None,
            bindings,
            extensions,
            user_state: UserStateStore::in_memory(),
            piano_roll_ui: PianoRollUiState::default(),
            graph_ui: GraphUiState::default(),
            playheads: HashMap::new(),
            playback: PlaybackEngine::new(),
            playback_end_secs: None,
            shell_panes: default_state(),
        }
    }

    /// Creates an app that loads and persists user state under `user_dir`.
    pub fn with_user_dir(
        user_dir: UserDir,
        extensions: ExtensionRuntime,
        bindings: Bindings,
    ) -> Self {
        let mut user_state = UserStateStore::open(user_dir);
        user_state.load_registered(extensions.user_state_bindings());
        let piano_roll_ui = restore_piano_roll_ui(&user_state, &extensions);
        let graph_ui = restore_graph_ui(&user_state, &extensions);
        let shell_panes = restore_shell_layout(&user_state, &extensions);
        let mut app = Self {
            documents: DocumentStore::default(),
            last_error: None,
            status_notice: None,
            bindings,
            extensions,
            user_state,
            piano_roll_ui,
            graph_ui,
            playheads: HashMap::new(),
            playback: PlaybackEngine::new(),
            playback_end_secs: None,
            shell_panes,
        };
        app.restore_open_documents();
        app
    }

    /// Shell pane-grid state.
    pub fn shell_panes(&self) -> &pane_grid::State<ShellPanelId> {
        &self.shell_panes
    }

    /// Graph-dag UI prefs.
    pub fn graph_ui(&self) -> &GraphUiState {
        &self.graph_ui
    }

    fn active_is_graph(&self) -> bool {
        self.documents
            .active_document()
            .and_then(|doc| doc.payload.graph())
            .is_some()
    }

    /// Whether the CPAL playback engine currently holds a running stream.
    pub fn is_playing(&self) -> bool {
        self.playback.is_playing()
    }

    /// Bucketed user-state store.
    pub fn user_state(&self) -> &UserStateStore {
        &self.user_state
    }

    /// Notation tool / snap state.
    pub fn piano_roll_ui(&self) -> &PianoRollUiState {
        &self.piano_roll_ui
    }

    /// Writes dirty user-state buckets immediately (tests and shutdown hooks).
    pub fn flush_user_state_now(&mut self) {
        if let Err(error) = self.user_state.flush_now() {
            warn!(%error, "failed to flush user state");
        }
    }

    /// Window title shown by the native shell.
    pub fn title(&self) -> String {
        match self.documents.active_document() {
            Some(doc) => {
                let dirty = if doc.dirty { " *" } else { "" };
                format!("Aura Editor — {}{dirty}", doc.title)
            }
            None => String::from("Aura Editor"),
        }
    }

    /// Open document store.
    pub fn documents(&self) -> &DocumentStore {
        &self.documents
    }

    /// Active input bindings.
    pub fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    /// Extension registries.
    pub fn extensions(&self) -> &ExtensionRuntime {
        &self.extensions
    }

    /// Last open error, if any.
    pub fn last_error(&self) -> Option<&OpenError> {
        self.last_error.as_ref()
    }

    /// Status line for empty state or errors.
    pub fn status(&self) -> String {
        if let Some(error) = &self.last_error {
            return error.status_message();
        }
        if let Some(notice) = &self.status_notice {
            return notice.clone();
        }
        if self.playback.is_playing() {
            return String::from("Playing");
        }
        if self.documents.active_document().is_some() {
            String::from("Ready")
        } else {
            String::from("Open a file or create a notation to begin")
        }
    }

    /// Keyboard subscription; while playing, also ticks for playhead + end detection.
    pub fn subscription(&self) -> Subscription<Message> {
        let keys = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat: false,
                ..
            } => Some(Message::KeyInput { key, modifiers }),
            _ => None,
        });
        if self.playback.is_playing() {
            let ticks = time::every(Duration::from_millis(33)).map(|_| Message::PlaybackTick);
            Subscription::batch(vec![keys, ticks])
        } else {
            keys
        }
    }

    /// Parked playhead beats for `document_id` (default 0).
    pub fn parked_playhead_beats(&self, document_id: DocumentId) -> f64 {
        self.playheads.get(&document_id).copied().unwrap_or(0.0)
    }

    /// Playhead beats to draw: live transport position while playing notation, else parked.
    pub fn display_playhead_beats(&self) -> f64 {
        let Some(doc) = self.documents.active_document() else {
            return 0.0;
        };
        let parked = self.parked_playhead_beats(doc.id);
        if !self.playback.is_playing() {
            return parked;
        }
        let Some(notation) = doc.payload.notation() else {
            return parked;
        };
        match secs_to_beats(notation, self.playback.position_secs()) {
            Ok(beats) => beats,
            Err(_) => parked,
        }
    }

    /// Applies a message to application state.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenClicked => return pick_open_file(),
            Message::OpenPath(path) => match self.extensions.open_document(&path) {
                Ok(opened) => {
                    self.last_error = None;
                    self.status_notice = None;
                    let path_for_error = path.clone();
                    if self
                        .documents
                        .open_or_activate_stacked(path, opened)
                        .is_none()
                    {
                        self.last_error = Some(OpenError::Invalid {
                            path: path_for_error,
                            reason: String::from("unsupported tip document payload"),
                        });
                        return Task::none();
                    }
                    self.piano_roll_ui.selected_note = None;
                    return self.sync_open_documents_task();
                }
                Err(error) => {
                    self.last_error = Some(error);
                }
            },
            Message::OpenFailed(error) => {
                self.last_error = Some(error);
            }
            Message::OpenCancelled => {}
            Message::SelectTab(id) => {
                self.documents.select(id);
                self.piano_roll_ui.selected_note = None;
                self.graph_ui.clear_selection();
                self.piano_roll_ui.playhead_beats = self.parked_playhead_beats(id);
                return self.sync_open_documents_task();
            }
            Message::CloseTab(id) => {
                self.documents.close(id);
                self.playheads.remove(&id);
                self.piano_roll_ui.selected_note = None;
                self.graph_ui.clear_selection();
                self.piano_roll_ui.playhead_beats = self
                    .documents
                    .active_id()
                    .map(|active| self.parked_playhead_beats(active))
                    .unwrap_or(0.0);
                return self.sync_open_documents_task();
            }
            Message::KeyInput { key, modifiers } => {
                if self.active_is_graph() && modifiers.is_empty() {
                    if let Some(command) = graph_tool_command_for_key(&key) {
                        return self.update(Message::Command(CommandId::new(command)));
                    }
                }
                if let Some(command) = self.bindings.resolve(&key, modifiers) {
                    return self.update(Message::Command(command));
                }
            }
            Message::Command(command) => {
                if !self.extensions.has_command(command.as_str()) {
                    return Task::none();
                }
                match command.as_str() {
                    CMD_DOCUMENT_OPEN => return pick_open_file(),
                    CMD_DOCUMENT_CLOSE_ACTIVE => {
                        if let Some(id) = self.documents.active_id() {
                            self.documents.close(id);
                            self.playheads.remove(&id);
                            self.piano_roll_ui.selected_note = None;
                            self.graph_ui.clear_selection();
                            self.piano_roll_ui.playhead_beats = self
                                .documents
                                .active_id()
                                .map(|active| self.parked_playhead_beats(active))
                                .unwrap_or(0.0);
                            return self.sync_open_documents_task();
                        }
                    }
                    CMD_DOCUMENT_NEW_NOTATION => {
                        self.last_error = None;
                        self.documents.create_untitled_notation();
                        self.piano_roll_ui.selected_note = None;
                        self.graph_ui.clear_selection();
                        return self.sync_open_documents_task();
                    }
                    CMD_DOCUMENT_NEW_GRAPH => {
                        self.last_error = None;
                        self.documents.create_untitled_graph();
                        self.graph_ui.clear_selection();
                        return self.sync_open_documents_task();
                    }
                    CMD_DOCUMENT_SAVE => return self.save_active(),
                    CMD_NOTATION_TOOL_DRAW => {
                        if self.active_is_graph() {
                            return Task::none();
                        }
                        self.piano_roll_ui.tool = NotationTool::Draw;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_NOTATION_TOOL_SELECT => {
                        if self.active_is_graph() {
                            return Task::none();
                        }
                        self.piano_roll_ui.tool = NotationTool::Select;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_NOTATION_TOOL_ERASE => {
                        if self.active_is_graph() {
                            return Task::none();
                        }
                        self.piano_roll_ui.tool = NotationTool::Erase;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_NOTATION_SNAP_TOGGLE => {
                        self.piano_roll_ui.snap_enabled = !self.piano_roll_ui.snap_enabled;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_GRAPH_TOOL_SELECT => {
                        self.graph_ui.tool = GraphTool::Select;
                        self.graph_ui.connect_draft = None;
                        return self.sync_graph_ui_task();
                    }
                    CMD_GRAPH_TOOL_ADD => {
                        self.graph_ui.tool = GraphTool::Add;
                        self.graph_ui.connect_draft = None;
                        return self.sync_graph_ui_task();
                    }
                    CMD_GRAPH_TOOL_CONNECT => {
                        self.graph_ui.tool = GraphTool::Connect;
                        self.graph_ui.connect_draft = None;
                        return self.sync_graph_ui_task();
                    }
                    CMD_GRAPH_TOOL_ERASE => {
                        self.graph_ui.tool = GraphTool::Erase;
                        self.graph_ui.connect_draft = None;
                        return self.sync_graph_ui_task();
                    }
                    CMD_GRAPH_DELETE_SELECTION => {
                        return self.delete_graph_selection();
                    }
                    CMD_PLAY_START => {
                        self.start_playback();
                    }
                    CMD_PLAY_STOP => {
                        self.stop_playback();
                    }
                    CMD_PLAY_TOGGLE => {
                        if self.playback.is_playing() {
                            self.stop_playback();
                        } else {
                            self.start_playback();
                        }
                    }
                    _ => {}
                }
            }
            Message::FlushUserState { generation } => {
                if let Err(error) = self.user_state.flush_if_generation(generation) {
                    warn!(%error, "failed to flush user state");
                }
            }
            Message::PianoRoll(edit) => {
                return self.apply_piano_roll_edit(edit);
            }
            Message::SnapResolutionSelected(resolution) => {
                self.piano_roll_ui.snap_resolution = resolution;
                return self.sync_piano_roll_ui_task();
            }
            Message::SaveToPath { document_id, path } => {
                return self.write_document(document_id, path);
            }
            Message::PlaybackTick => {
                if let Some(end) = self.playback_end_secs {
                    if self.playback.position_secs() >= end {
                        self.stop_playback();
                    }
                }
            }
            Message::Graph(edit) => {
                return self.apply_graph_edit(edit);
            }
            Message::GraphAddTypeSelected(node_type) => {
                self.graph_ui.add_node_type = node_type;
                self.graph_ui.tool = GraphTool::Add;
                self.graph_ui.connect_draft = None;
                return self.sync_graph_ui_task();
            }
            Message::GraphConnectionModeSelected(mode) => {
                self.graph_ui.connection_mode = mode;
                self.graph_ui.connect_draft = None;
                return self.sync_graph_ui_task();
            }
            Message::GraphRenameInput(value) => {
                self.graph_ui.rename_buffer = value;
            }
            Message::GraphRenameCommit => {
                return self.commit_graph_rename();
            }
            Message::GraphSelectedTypeChanged(node_type) => {
                return self.change_selected_node_type(node_type);
            }
            Message::ShellPaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.shell_panes.drop(pane, target);
                return self.sync_shell_layout_task();
            }
            Message::ShellPaneDragged(_) => {}
            Message::ShellPaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.shell_panes.resize(split, ratio);
                return self.sync_shell_layout_task();
            }
        }
        Task::none()
    }

    fn start_playback(&mut self) {
        let Some(doc) = self.documents.active_document() else {
            self.status_notice = Some(String::from("Playback requires an open document"));
            return;
        };
        let sample_rate = SampleRate::RATE_44100;

        if let Some(notation) = doc.payload.notation() {
            let playhead_beats = self.parked_playhead_beats(doc.id);
            let start_secs = match playhead_start_secs(notation, playhead_beats) {
                Ok(secs) => secs,
                Err(error) => {
                    self.status_notice = Some(format!("Playback failed: {error}"));
                    return;
                }
            };
            let (sampler, duration_secs) =
                match prepare_notation_playback(notation, sample_rate) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        self.status_notice = Some(format!("Playback failed: {error}"));
                        return;
                    }
                };
            match self
                .playback
                .start_at(Arc::from(sampler), sample_rate, start_secs)
            {
                Ok(()) => {
                    self.last_error = None;
                    self.status_notice = None;
                    self.playback_end_secs = Some(duration_secs.max(start_secs));
                }
                Err(error) => {
                    self.status_notice = Some(format!("Playback failed: {error}"));
                    self.playback_end_secs = None;
                }
            }
            return;
        }

        let Some(graph) = doc.payload.graph() else {
            self.status_notice =
                Some(String::from("Playback requires an Imp graph or notation tab"));
            return;
        };
        let neighbor_hint = doc.path.as_deref();

        let sampler = match prepare_session_playback(graph, neighbor_hint, sample_rate) {
            Ok((_node_id, sampler)) => sampler,
            Err(error) => {
                self.status_notice = Some(format!("Playback failed: {error}"));
                return;
            }
        };

        match self.playback.start(Arc::from(sampler), sample_rate) {
            Ok(()) => {
                self.last_error = None;
                self.status_notice = None;
                self.playback_end_secs = None;
            }
            Err(error) => {
                self.status_notice = Some(format!("Playback failed: {error}"));
                self.playback_end_secs = None;
            }
        }
    }

    fn stop_playback(&mut self) {
        self.playback.stop();
        self.playback_end_secs = None;
        if self
            .status_notice
            .as_deref()
            .is_some_and(|notice| notice == "Playing" || notice.starts_with("Playback "))
        {
            self.status_notice = None;
        }
    }

    fn apply_piano_roll_edit(&mut self, edit: PianoRollEdit) -> Task<Message> {
        let document_id = match &edit {
            PianoRollEdit::Draw { document_id, .. }
            | PianoRollEdit::Select { document_id, .. }
            | PianoRollEdit::MoveSelected { document_id, .. }
            | PianoRollEdit::ResizeSelected { document_id, .. }
            | PianoRollEdit::Erase { document_id, .. }
            | PianoRollEdit::SetPlayhead { document_id, .. } => *document_id,
        };

        if let PianoRollEdit::SetPlayhead { beats, .. } = edit {
            self.playheads.insert(document_id, beats.max(0.0));
            self.piano_roll_ui.playhead_beats = beats.max(0.0);
            return Task::none();
        }

        if matches!(edit, PianoRollEdit::Select { .. }) {
            if let PianoRollEdit::Select { note_index, .. } = edit {
                self.piano_roll_ui.selected_note = note_index;
            }
            return Task::none();
        }

        let Some(doc) = self.documents.get_mut(document_id) else {
            return Task::none();
        };
        let Some(score) = doc.payload.notation_mut() else {
            return Task::none();
        };
        if apply_edit(score, &edit) {
            doc.dirty = true;
            if let PianoRollEdit::Erase { note_index, .. } = edit {
                if self.piano_roll_ui.selected_note == Some(note_index) {
                    self.piano_roll_ui.selected_note = None;
                } else if let Some(selected) = self.piano_roll_ui.selected_note {
                    if selected > note_index {
                        self.piano_roll_ui.selected_note = Some(selected - 1);
                    }
                }
            }
        }
        Task::none()
    }

    fn save_active(&mut self) -> Task<Message> {
        let Some(doc) = self.documents.active_document() else {
            return Task::none();
        };
        let can_save = doc.payload.notation().is_some() || doc.payload.graph().is_some();
        if !can_save {
            self.last_error = Some(OpenError::Invalid {
                path: doc.path.clone().unwrap_or_else(|| PathBuf::from("unsaved")),
                reason: String::from("only notation and Imp graph documents can be saved"),
            });
            return Task::none();
        }
        let document_id = doc.id;
        if let Some(path) = doc.path.clone() {
            return self.write_document(document_id, path);
        }
        pick_save_file(document_id)
    }

    fn write_document(&mut self, document_id: DocumentId, path: PathBuf) -> Task<Message> {
        let Some(doc) = self.documents.get_mut(document_id) else {
            return Task::none();
        };
        let json = if let Some(score) = doc.payload.notation() {
            match score.to_pretty_json() {
                Ok(json) => json,
                Err(error) => {
                    self.last_error = Some(OpenError::Invalid {
                        path: path.clone(),
                        reason: error.to_string(),
                    });
                    return Task::none();
                }
            }
        } else if let Some(graph) = doc.payload.graph() {
            match graph_to_aura_json(graph) {
                Ok(json) => json,
                Err(error) => {
                    self.last_error = Some(OpenError::Invalid {
                        path: path.clone(),
                        reason: error.to_string(),
                    });
                    return Task::none();
                }
            }
        } else {
            return Task::none();
        };
        if let Err(error) = std::fs::write(&path, json) {
            self.last_error = Some(OpenError::Io {
                path: path.clone(),
                message: error.to_string(),
            });
            return Task::none();
        }
        self.last_error = None;
        self.documents.set_path_and_clean(document_id, path);
        self.sync_open_documents_task()
    }

    fn graph_ui_binding(&self) -> Option<&UserStateBinding> {
        self.extensions
            .user_state_bindings()
            .iter()
            .find(|binding| binding.entry_id == USER_STATE_GRAPH_DAG_UI)
    }

    fn sync_graph_ui_task(&mut self) -> Task<Message> {
        let Some(binding) = self.graph_ui_binding().cloned() else {
            return Task::none();
        };
        let entry = self.graph_ui.to_entry();
        let generation = match self
            .user_state
            .set_entry(&binding.bucket, &binding.entry_id, &entry)
        {
            Ok(generation) => generation,
            Err(error) => {
                warn!(%error, "failed to update graph-dag-ui user state");
                return Task::none();
            }
        };
        self.schedule_user_state_flush(generation)
    }

    fn apply_graph_edit(&mut self, edit: GraphEdit) -> Task<Message> {
        match edit {
            GraphEdit::SelectNode {
                document_id,
                node_id,
            } => {
                if self.documents.active_id() != Some(document_id) {
                    return Task::none();
                }
                self.graph_ui.select_node(node_id);
            }
            GraphEdit::SelectEdge {
                document_id,
                edge_id,
            } => {
                if self.documents.active_id() != Some(document_id) {
                    return Task::none();
                }
                self.graph_ui.select_edge(edge_id);
            }
            GraphEdit::AddNode { document_id } => {
                let Some(doc) = self.documents.get_mut(document_id) else {
                    return Task::none();
                };
                let Some(graph) = doc.payload.graph_mut() else {
                    return Task::none();
                };
                let id = unique_node_id(graph, &self.graph_ui.add_node_type);
                if let Err(error) = add_node(graph, id.clone(), self.graph_ui.add_node_type.clone())
                {
                    self.status_notice = Some(error.to_string());
                    return Task::none();
                }
                doc.dirty = true;
                self.status_notice = None;
                self.graph_ui.select_node(Some(id));
            }
            GraphEdit::BeginConnect {
                document_id,
                from_node,
                from_port,
            } => {
                if self.documents.active_id() != Some(document_id) {
                    return Task::none();
                }
                self.graph_ui.connect_draft = Some(ConnectDraft {
                    from_node,
                    from_port,
                });
            }
            GraphEdit::CancelConnect { document_id } => {
                if self.documents.active_id() == Some(document_id) {
                    self.graph_ui.connect_draft = None;
                }
            }
            GraphEdit::ConnectNodes {
                document_id,
                from,
                to,
            } => {
                let neighbor = self
                    .documents
                    .get_mut(document_id)
                    .and_then(|doc| doc.path.clone());
                let registry = match node_type_registry(neighbor.as_deref()) {
                    Ok(registry) => registry,
                    Err(error) => {
                        self.status_notice = Some(error.to_string());
                        self.graph_ui.connect_draft = None;
                        return Task::none();
                    }
                };
                let Some(doc) = self.documents.get_mut(document_id) else {
                    return Task::none();
                };
                let Some(graph) = doc.payload.graph_mut() else {
                    return Task::none();
                };
                match auto_pair_ports(&registry, graph, &from, &to) {
                    Ok((from_port, to_port)) => {
                        if let Err(error) = add_edge(graph, from, from_port, to, to_port) {
                            self.status_notice = Some(error.to_string());
                        } else {
                            doc.dirty = true;
                            self.status_notice = None;
                        }
                    }
                    Err(error) => {
                        self.status_notice = Some(error.to_string());
                    }
                }
                self.graph_ui.connect_draft = None;
            }
            GraphEdit::ConnectPorts {
                document_id,
                from_node,
                from_port,
                to_node,
                to_port,
            } => {
                let Some(doc) = self.documents.get_mut(document_id) else {
                    return Task::none();
                };
                let Some(graph) = doc.payload.graph_mut() else {
                    return Task::none();
                };
                if let Err(error) = add_edge(graph, from_node, from_port, to_node, to_port) {
                    self.status_notice = Some(error.to_string());
                } else {
                    doc.dirty = true;
                    self.status_notice = None;
                }
                self.graph_ui.connect_draft = None;
            }
            GraphEdit::EraseNode {
                document_id,
                node_id,
            } => {
                let Some(doc) = self.documents.get_mut(document_id) else {
                    return Task::none();
                };
                let Some(graph) = doc.payload.graph_mut() else {
                    return Task::none();
                };
                if let Err(error) = remove_node(graph, &node_id) {
                    self.status_notice = Some(error.to_string());
                } else {
                    doc.dirty = true;
                    self.status_notice = None;
                    if self.graph_ui.selected_node.as_deref() == Some(node_id.as_str()) {
                        self.graph_ui.clear_selection();
                    }
                }
            }
            GraphEdit::EraseEdge {
                document_id,
                edge_id,
            } => {
                let Some(doc) = self.documents.get_mut(document_id) else {
                    return Task::none();
                };
                let Some(graph) = doc.payload.graph_mut() else {
                    return Task::none();
                };
                if let Err(error) = remove_edge(graph, &edge_id) {
                    self.status_notice = Some(error.to_string());
                } else {
                    doc.dirty = true;
                    self.status_notice = None;
                    if self.graph_ui.selected_edge.as_deref() == Some(edge_id.as_str()) {
                        self.graph_ui.clear_selection();
                    }
                }
            }
            GraphEdit::EraseEdgesBetween {
                document_id,
                a,
                b,
            } => {
                let Some(doc) = self.documents.get_mut(document_id) else {
                    return Task::none();
                };
                let Some(graph) = doc.payload.graph_mut() else {
                    return Task::none();
                };
                let removed = remove_edges_between(graph, &a, &b);
                if removed > 0 {
                    doc.dirty = true;
                    self.status_notice = None;
                    self.graph_ui.clear_selection();
                }
            }
        }
        Task::none()
    }

    fn delete_graph_selection(&mut self) -> Task<Message> {
        let Some(doc) = self.documents.active_document() else {
            return Task::none();
        };
        if doc.payload.graph().is_none() {
            return Task::none();
        }
        let document_id = doc.id;
        if let Some(node_id) = self.graph_ui.selected_node.clone() {
            return self.apply_graph_edit(GraphEdit::EraseNode {
                document_id,
                node_id,
            });
        }
        if let Some(edge_id) = self.graph_ui.selected_edge.clone() {
            return self.apply_graph_edit(GraphEdit::EraseEdge {
                document_id,
                edge_id,
            });
        }
        Task::none()
    }

    fn commit_graph_rename(&mut self) -> Task<Message> {
        let Some(old_id) = self.graph_ui.selected_node.clone() else {
            return Task::none();
        };
        let new_id = self.graph_ui.rename_buffer.trim().to_string();
        let Some(doc) = self.documents.active_document_mut() else {
            return Task::none();
        };
        let Some(graph) = doc.payload.graph_mut() else {
            return Task::none();
        };
        match rename_node(graph, &old_id, new_id.clone()) {
            Ok(id) => {
                doc.dirty = true;
                self.status_notice = None;
                self.graph_ui.select_node(Some(id));
            }
            Err(error) => {
                self.status_notice = Some(error.to_string());
                self.graph_ui.rename_buffer = old_id;
            }
        }
        Task::none()
    }

    fn change_selected_node_type(&mut self, node_type: String) -> Task<Message> {
        let Some(node_id) = self.graph_ui.selected_node.clone() else {
            return Task::none();
        };
        let Some(doc) = self.documents.active_document_mut() else {
            return Task::none();
        };
        let Some(graph) = doc.payload.graph_mut() else {
            return Task::none();
        };
        if let Err(error) = set_node_type(graph, &node_id, node_type) {
            self.status_notice = Some(error.to_string());
        } else {
            doc.dirty = true;
            self.status_notice = None;
        }
        Task::none()
    }

    fn open_documents_binding(&self) -> Option<&UserStateBinding> {
        self.extensions
            .user_state_bindings()
            .iter()
            .find(|binding| binding.entry_id == USER_STATE_OPEN_DOCUMENTS)
    }

    fn piano_roll_ui_binding(&self) -> Option<&UserStateBinding> {
        self.extensions
            .user_state_bindings()
            .iter()
            .find(|binding| binding.entry_id == USER_STATE_PIANO_ROLL_UI)
    }

    fn shell_layout_binding(&self) -> Option<&UserStateBinding> {
        self.extensions
            .user_state_bindings()
            .iter()
            .find(|binding| binding.entry_id == USER_STATE_SHELL_LAYOUT)
    }

    fn sync_shell_layout_task(&mut self) -> Task<Message> {
        let Some(binding) = self.shell_layout_binding().cloned() else {
            return Task::none();
        };
        let Some(entry) = ShellLayoutEntry::from_state(&self.shell_panes) else {
            warn!("failed to serialize shell layout; skipping persist");
            return Task::none();
        };
        let generation = match self
            .user_state
            .set_entry(&binding.bucket, &binding.entry_id, &entry)
        {
            Ok(generation) => generation,
            Err(error) => {
                warn!(%error, "failed to update shell-layout user state");
                return Task::none();
            }
        };
        self.schedule_user_state_flush(generation)
    }

    fn restore_open_documents(&mut self) {
        let Some(binding) = self.open_documents_binding().cloned() else {
            return;
        };
        let Some(entry) = self
            .user_state
            .get_entry::<OpenDocumentsEntry>(&binding.bucket, &binding.entry_id)
        else {
            return;
        };

        for path in entry.open_paths {
            match self.extensions.open_document(&path) {
                Ok(opened) => {
                    if self
                        .documents
                        .open_or_activate_stacked(path.clone(), opened)
                        .is_none()
                    {
                        warn!(
                            path = %path.display(),
                            "skipped restoring document; unsupported tip payload"
                        );
                    }
                }
                Err(error) => {
                    warn!(
                        path = %path.display(),
                        %error,
                        "skipped restoring document"
                    );
                }
            }
        }

        if let Some(active_path) = entry.active_path {
            if let Some(doc) = self.documents.find_by_path(&active_path) {
                let id = doc.id;
                self.documents.select(id);
            }
        }
    }

    fn sync_open_documents_task(&mut self) -> Task<Message> {
        let Some(binding) = self.open_documents_binding().cloned() else {
            return Task::none();
        };
        let entry = OpenDocumentsEntry {
            open_paths: self
                .documents
                .documents()
                .iter()
                .filter_map(|doc| doc.path.clone())
                .collect(),
            active_path: self
                .documents
                .active_document()
                .and_then(|doc| doc.path.clone()),
        };
        let generation = match self
            .user_state
            .set_entry(&binding.bucket, &binding.entry_id, &entry)
        {
            Ok(generation) => generation,
            Err(error) => {
                warn!(%error, "failed to update open-documents user state");
                return Task::none();
            }
        };
        self.schedule_user_state_flush(generation)
    }

    fn sync_piano_roll_ui_task(&mut self) -> Task<Message> {
        let Some(binding) = self.piano_roll_ui_binding().cloned() else {
            return Task::none();
        };
        let entry = self.piano_roll_ui.to_entry();
        let generation = match self
            .user_state
            .set_entry(&binding.bucket, &binding.entry_id, &entry)
        {
            Ok(generation) => generation,
            Err(error) => {
                warn!(%error, "failed to update piano-roll-ui user state");
                return Task::none();
            }
        };
        self.schedule_user_state_flush(generation)
    }

    fn schedule_user_state_flush(&self, generation: u64) -> Task<Message> {
        if !self.user_state.persists_to_disk() {
            return Task::none();
        }
        Task::perform(
            async move {
                tokio::time::sleep(DEBOUNCE).await;
            },
            move |_| Message::FlushUserState { generation },
        )
    }

    /// Builds the multi-document shell view.
    pub fn view(&self) -> Element<'_, Message> {
        let panes = pane_grid(&self.shell_panes, |_pane, panel_id, _maximized| {
            let title = pane_grid::TitleBar::new(text(panel_id.title()).size(12)).padding(4);
            let body = match panel_id {
                ShellPanelId::Status => row![text(self.status())].padding(8).into(),
                ShellPanelId::ViewToolbar => self.view_toolbar_pane(),
                ShellPanelId::Documents => {
                    column![self.tab_bar(), self.document_panel()].spacing(0).into()
                }
                ShellPanelId::Properties => self.properties_pane(),
            };
            pane_grid::Content::new(body).title_bar(title)
        })
        .width(Fill)
        .height(Fill)
        .spacing(4)
        .on_drag(Message::ShellPaneDragged)
        .on_resize(8, Message::ShellPaneResized);

        column![menu_bar::global_menu_bar(), panes]
            .spacing(0)
            .into()
    }

    fn view_toolbar_pane(&self) -> Element<'_, Message> {
        if self
            .documents
            .active_document()
            .and_then(|doc| doc.payload.notation())
            .is_some()
        {
            row![piano_roll_tool_bar(&self.piano_roll_ui)]
                .spacing(6)
                .padding([0, 8])
                .into()
        } else if self.active_is_graph() {
            row![self.graph_tool_bar()]
                .spacing(6)
                .padding([0, 8])
                .into()
        } else {
            row![].padding([0, 8]).into()
        }
    }

    fn graph_tool_bar(&self) -> Element<'_, Message> {
        let select_label = if self.graph_ui.tool == GraphTool::Select {
            "[Select]"
        } else {
            "Select"
        };
        let add_label = if self.graph_ui.tool == GraphTool::Add {
            "[Add]"
        } else {
            "Add"
        };
        let connect_label = if self.graph_ui.tool == GraphTool::Connect {
            "[Connect]"
        } else {
            "Connect"
        };
        let erase_label = if self.graph_ui.tool == GraphTool::Erase {
            "[Erase]"
        } else {
            "Erase"
        };

        let type_ids = available_node_types(
            self.documents
                .active_document()
                .and_then(|doc| doc.path.as_deref()),
        );
        let add_type = self.graph_ui.add_node_type.clone();

        let left = row![
            button(select_label).on_press(Message::Command(CommandId::new(CMD_GRAPH_TOOL_SELECT))),
            button(add_label).on_press(Message::Command(CommandId::new(CMD_GRAPH_TOOL_ADD))),
            pick_list(
                type_ids,
                Some(add_type),
                Message::GraphAddTypeSelected,
            ),
            button(connect_label)
                .on_press(Message::Command(CommandId::new(CMD_GRAPH_TOOL_CONNECT))),
            button(erase_label).on_press(Message::Command(CommandId::new(CMD_GRAPH_TOOL_ERASE))),
        ]
        .spacing(6);

        let mode_label = match self.graph_ui.connection_mode {
            ConnectionMode::Simple => "Simple",
            ConnectionMode::Precise => "Precise",
        };
        let options_menu = Menu::new(menu_items!(
            (button(text(format!("Connection: {mode_label} (toggle)")).width(Length::Fill))
                .on_press(Message::GraphConnectionModeSelected(
                    match self.graph_ui.connection_mode {
                        ConnectionMode::Simple => ConnectionMode::Precise,
                        ConnectionMode::Precise => ConnectionMode::Simple,
                    },
                ))
                .width(Length::Fill)),
            (button(text("Simple").width(Length::Fill))
                .on_press(Message::GraphConnectionModeSelected(ConnectionMode::Simple))
                .width(Length::Fill)),
            (button(text("Precise").width(Length::Fill))
                .on_press(Message::GraphConnectionModeSelected(ConnectionMode::Precise))
                .width(Length::Fill)),
        ))
        .max_width(220.0);

        let options = MenuBar::new(menu_items!(
            (button(text("Graph view options")), options_menu),
        ))
        .padding(0)
        .spacing(0)
        .close_on_item_click(true)
        .close_on_background_click(true);

        row![left, Space::new().width(Fill), options]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn properties_pane(&self) -> Element<'_, Message> {
        if !self.active_is_graph() {
            return column![text("No graph selection")]
                .spacing(8)
                .padding(8)
                .into();
        }

        let Some(node_id) = &self.graph_ui.selected_node else {
            return column![text("Select a node to edit properties")]
                .spacing(8)
                .padding(8)
                .into();
        };

        let type_ids = available_node_types(
            self.documents
                .active_document()
                .and_then(|doc| doc.path.as_deref()),
        );
        let selected_type = self
            .documents
            .active_document()
            .and_then(|doc| doc.payload.graph())
            .and_then(|graph| graph.nodes.get(node_id))
            .map(|node| node.node_type.clone());

        column![
            text(format!("Node: {node_id}")).size(14),
            text("Id:"),
            text_input("node id", &self.graph_ui.rename_buffer)
                .on_input(Message::GraphRenameInput)
                .on_submit(Message::GraphRenameCommit)
                .width(Length::Fill),
            button("Rename").on_press(Message::GraphRenameCommit),
            text("Type:"),
            pick_list(type_ids, selected_type, Message::GraphSelectedTypeChanged),
        ]
        .spacing(8)
        .padding(8)
        .into()
    }

    fn tab_bar(&self) -> Element<'_, Message> {
        if self.documents.documents().is_empty() {
            return Space::new().height(0).into();
        }

        let mut tabs = row![].spacing(4).padding([0, 8]);
        for doc in self.documents.documents() {
            let active = self.documents.active_id() == Some(doc.id);
            let dirty = if doc.dirty { "*" } else { "" };
            let label = if active {
                format!("[{}{dirty}]", doc.title)
            } else {
                format!("{}{dirty}", doc.title)
            };
            tabs = tabs.push(
                row![
                    button(text(label)).on_press(Message::SelectTab(doc.id)),
                    button(text("×")).on_press(Message::CloseTab(doc.id)),
                ]
                .spacing(2),
            );
        }
        tabs.into()
    }

    fn document_panel(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if let Some(doc) = self.documents.active_document()
        {
            match self.extensions.resolve_view_for_chain(&doc.roles) {
                Some((role, VIEW_GRAPH_DAG)) => match doc.payload.graph() {
                    Some(graph) => {
                        let ports = build_port_catalog(graph, doc.path.as_deref());
                        graph_canvas(doc.id, graph, &self.graph_ui, ports)
                    }
                    None => view_error_placeholder(format!(
                        "View `{VIEW_GRAPH_DAG}` requires an Imp graph tip (role `{role}`)"
                    )),
                },
                Some((role, VIEW_PIANO_ROLL)) => match doc.payload.notation() {
                    Some(notation) => piano_roll_panel(
                        doc.id,
                        notation,
                        &self.piano_roll_ui,
                        self.display_playhead_beats(),
                    ),
                    None => view_error_placeholder(format!(
                        "View `{VIEW_PIANO_ROLL}` requires a notation tip (role `{role}`)"
                    )),
                },
                Some((role, other)) => view_error_placeholder(format!(
                    "No host adapter for view `{other}` (role `{role}`)"
                )),
                None => {
                    let hint = doc
                        .roles
                        .iter()
                        .rev()
                        .flatten()
                        .next()
                        .map(|role| format!("No enabled view for role `{role}`"))
                        .unwrap_or_else(|| {
                            String::from("No enabled view for this document type chain")
                        });
                    view_error_placeholder(hint)
                }
            }
        } else {
            container(text("Open a file or create a notation to begin"))
                .center_x(Fill)
                .center_y(Fill)
                .width(Fill)
                .height(Fill)
                .into()
        };

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(8)
            .into()
    }
}

fn piano_roll_tool_bar(ui: &PianoRollUiState) -> Element<'_, Message> {
    let draw_label = if ui.tool == NotationTool::Draw {
        "[Draw]"
    } else {
        "Draw"
    };
    let select_label = if ui.tool == NotationTool::Select {
        "[Select]"
    } else {
        "Select"
    };
    let erase_label = if ui.tool == NotationTool::Erase {
        "[Erase]"
    } else {
        "Erase"
    };
    let snap_label = if ui.snap_enabled {
        "[Snap]"
    } else {
        "Snap"
    };

    row![
        button(draw_label).on_press(Message::Command(CommandId::new(CMD_NOTATION_TOOL_DRAW))),
        button(select_label).on_press(Message::Command(CommandId::new(CMD_NOTATION_TOOL_SELECT))),
        button(erase_label).on_press(Message::Command(CommandId::new(CMD_NOTATION_TOOL_ERASE))),
        button(snap_label).on_press(Message::Command(CommandId::new(CMD_NOTATION_SNAP_TOGGLE))),
        text("Res:"),
        pick_list(
            SnapResolution::ALL.to_vec(),
            Some(ui.snap_resolution),
            Message::SnapResolutionSelected,
        ),
    ]
    .spacing(6)
    .into()
}

fn restore_piano_roll_ui(
    store: &UserStateStore,
    extensions: &ExtensionRuntime,
) -> PianoRollUiState {
    let Some(binding) = extensions
        .user_state_bindings()
        .iter()
        .find(|binding| binding.entry_id == USER_STATE_PIANO_ROLL_UI)
    else {
        return PianoRollUiState::default();
    };
    store
        .get_entry::<PianoRollUiEntry>(&binding.bucket, &binding.entry_id)
        .map(|entry| PianoRollUiState::from_entry(&entry))
        .unwrap_or_default()
}

fn restore_graph_ui(store: &UserStateStore, extensions: &ExtensionRuntime) -> GraphUiState {
    let Some(binding) = extensions
        .user_state_bindings()
        .iter()
        .find(|binding| binding.entry_id == USER_STATE_GRAPH_DAG_UI)
    else {
        return GraphUiState::default();
    };
    store
        .get_entry::<GraphUiEntry>(&binding.bucket, &binding.entry_id)
        .map(|entry| GraphUiState::from_entry(&entry))
        .unwrap_or_default()
}

fn restore_shell_layout(
    store: &UserStateStore,
    extensions: &ExtensionRuntime,
) -> pane_grid::State<ShellPanelId> {
    let Some(binding) = extensions
        .user_state_bindings()
        .iter()
        .find(|binding| binding.entry_id == USER_STATE_SHELL_LAYOUT)
    else {
        return default_state();
    };
    let entry = store.get_entry::<ShellLayoutEntry>(&binding.bucket, &binding.entry_id);
    state_from_entry(entry.as_ref())
}

fn graph_tool_command_for_key(key: &Key) -> Option<&'static str> {
    match key {
        Key::Character(text) => match text.as_str() {
            "s" | "S" => Some(CMD_GRAPH_TOOL_SELECT),
            "a" | "A" => Some(CMD_GRAPH_TOOL_ADD),
            "c" | "C" => Some(CMD_GRAPH_TOOL_CONNECT),
            "e" | "E" => Some(CMD_GRAPH_TOOL_ERASE),
            _ => None,
        },
        Key::Named(Named::Delete) | Key::Named(Named::Backspace) => {
            Some(CMD_GRAPH_DELETE_SELECTION)
        }
        _ => None,
    }
}

fn node_type_registry(
    neighbor_hint: Option<&std::path::Path>,
) -> Result<aura_imp::Registry, String> {
    match playback_registry(neighbor_hint) {
        Ok(registry) => Ok(registry),
        Err(_) => aura_registry().map_err(|error| error.to_string()),
    }
}

fn available_node_types(neighbor_hint: Option<&std::path::Path>) -> Vec<String> {
    let Ok(registry) = node_type_registry(neighbor_hint) else {
        return vec![String::from("sine"), String::from("output"), String::from("parameter")];
    };
    let mut ids: Vec<String> = list_node_types(&registry)
        .into_iter()
        .map(|def| def.id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        ids.push(String::from("sine"));
    }
    ids
}

fn build_port_catalog(
    graph: &imp_core_types::Graph,
    neighbor_hint: Option<&std::path::Path>,
) -> PortCatalog {
    let mut catalog = PortCatalog::new();
    let Ok(registry) = node_type_registry(neighbor_hint) else {
        return catalog;
    };
    for node in graph.nodes.values() {
        let mut info = NodePortInfo::default();
        if let Some(def) = get_node_type(&registry, &node.node_type) {
            info.inputs = def.inputs.keys().cloned().collect();
            info.outputs = def.outputs.keys().cloned().collect();
        }
        catalog.insert(node.id.clone(), info);
    }
    catalog
}

fn view_error_placeholder(message: String) -> Element<'static, Message> {
    container(text(message))
        .center_x(Fill)
        .center_y(Fill)
        .width(Fill)
        .height(Fill)
        .into()
}

fn pick_open_file() -> Task<Message> {
    Task::perform(pick_open_file_async(), |path| match path {
        Some(path) => Message::OpenPath(path),
        None => Message::OpenCancelled,
    })
}

async fn pick_open_file_async() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("Aura JSON", &["json"])
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}

fn pick_save_file(document_id: DocumentId) -> Task<Message> {
    Task::perform(pick_save_file_async(), move |path| match path {
        Some(path) => Message::SaveToPath { document_id, path },
        None => Message::OpenCancelled,
    })
}

async fn pick_save_file_async() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("Aura JSON", &["json"])
        .set_file_name("untitled.json")
        .save_file()
        .await
        .map(|file| file.path().to_path_buf())
}
