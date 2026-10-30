//! Aura Editor application library.
//!
//! Keeps Iced application logic out of `main` so unit and headless functional
//! tests can exercise state and view without opening a real window.

mod bindings;
mod command;
mod document;
mod extensions;
mod graph_file;
mod graph_layout;
mod graph_view;
mod logging;
mod menu_bar;
mod piano_roll;
mod piano_roll_ui;
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
pub use graph_file::OpenError;
pub use graph_layout::{layout_graph, GraphLayout, LaidOutNode};
pub use graph_view::{
    center_translation, clamp_scale, pan_by, zoom_at_point, GraphView,
};
pub use logging::init_logging;
pub use piano_roll_ui::{PianoRollUiEntry, PianoRollUiState};
pub use piano_roll::{apply_edit, piano_roll_panel, PianoRollEdit, PITCH_MAX, PITCH_MIN};
pub use user_state::{
    write_atomic, OpenDocumentsEntry, UserDir, UserStateError, UserStateStore, BUCKET_SCHEMA_VERSION,
    DEBOUNCE, DEFAULT_USER_DIR_NAME, USER_DIR_ENV,
};

pub use aura_ext_notation::{
    semitone_label, snap_beats, Notation, NotationNote, NotationTool, SnapResolution,
};
pub use aura_interfaces::{
    CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_NEW_NOTATION, CMD_DOCUMENT_OPEN, CMD_DOCUMENT_SAVE,
    CMD_NOTATION_SNAP_TOGGLE, CMD_NOTATION_TOOL_DRAW, CMD_NOTATION_TOOL_ERASE,
    CMD_NOTATION_TOOL_SELECT, DOC_TYPE_AURA_JSON, DOC_TYPE_IMP_GRAPH, DOC_TYPE_IMP_GRAPH_LIBRARY,
    DOC_TYPE_JSON, DOC_TYPE_NOTATION, ROLE_IMP_GRAPH, ROLE_JSON, ROLE_NOTATION,
    USER_STATE_BUCKET_DOCUMENTS, USER_STATE_BUCKET_NOTATION, USER_STATE_PIANO_ROLL_UI,
    USER_STATE_OPEN_DOCUMENTS, VIEW_GRAPH_DAG, VIEW_PIANO_ROLL,
};

use std::path::PathBuf;

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{button, column, container, pick_list, row, text, Space};
use iced::{Element, Fill, Subscription, Task};
use tracing::warn;

use crate::graph_view::graph_canvas;

/// Root application state for the editor shell.
#[derive(Debug, Clone)]
pub struct App {
    documents: DocumentStore,
    /// Last open/load/save error shown in the status banner.
    last_error: Option<OpenError>,
    /// Resolved input bindings (may be empty).
    bindings: Bindings,
    /// Extension command/view/document-type registries.
    extensions: ExtensionRuntime,
    /// Bucketed ephemeral user state.
    user_state: UserStateStore,
    /// Notation tool / snap prefs (persisted when enabled).
    piano_roll_ui: PianoRollUiState,
}

/// Messages produced by the multi-document shell.
#[derive(Debug, Clone, PartialEq)]
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
}

impl From<PianoRollEdit> for Message {
    fn from(edit: PianoRollEdit) -> Self {
        Self::PianoRoll(edit)
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
            bindings,
            extensions,
            user_state: UserStateStore::in_memory(),
            piano_roll_ui: PianoRollUiState::default(),
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
        let mut app = Self {
            documents: DocumentStore::default(),
            last_error: None,
            bindings,
            extensions,
            user_state,
            piano_roll_ui,
        };
        app.restore_open_documents();
        app
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
        if self.documents.active_document().is_some() {
            String::from("Ready")
        } else {
            String::from("Open a file or create a notation to begin")
        }
    }

    /// Keyboard subscription: uncaptured key presses become [`Message::KeyInput`].
    pub fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat: false,
                ..
            } => Some(Message::KeyInput { key, modifiers }),
            _ => None,
        })
    }

    /// Applies a message to application state.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenClicked => return pick_open_file(),
            Message::OpenPath(path) => match self.extensions.open_document(&path) {
                Ok(opened) => {
                    self.last_error = None;
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
                return self.sync_open_documents_task();
            }
            Message::CloseTab(id) => {
                self.documents.close(id);
                self.piano_roll_ui.selected_note = None;
                return self.sync_open_documents_task();
            }
            Message::KeyInput { key, modifiers } => {
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
                            self.piano_roll_ui.selected_note = None;
                            return self.sync_open_documents_task();
                        }
                    }
                    CMD_DOCUMENT_NEW_NOTATION => {
                        self.last_error = None;
                        self.documents.create_untitled_notation();
                        self.piano_roll_ui.selected_note = None;
                        return self.sync_open_documents_task();
                    }
                    CMD_DOCUMENT_SAVE => return self.save_active(),
                    CMD_NOTATION_TOOL_DRAW => {
                        self.piano_roll_ui.tool = NotationTool::Draw;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_NOTATION_TOOL_SELECT => {
                        self.piano_roll_ui.tool = NotationTool::Select;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_NOTATION_TOOL_ERASE => {
                        self.piano_roll_ui.tool = NotationTool::Erase;
                        return self.sync_piano_roll_ui_task();
                    }
                    CMD_NOTATION_SNAP_TOGGLE => {
                        self.piano_roll_ui.snap_enabled = !self.piano_roll_ui.snap_enabled;
                        return self.sync_piano_roll_ui_task();
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
                return self.write_notation_document(document_id, path);
            }
        }
        Task::none()
    }

    fn apply_piano_roll_edit(&mut self, edit: PianoRollEdit) -> Task<Message> {
        let document_id = match &edit {
            PianoRollEdit::Draw { document_id, .. }
            | PianoRollEdit::Select { document_id, .. }
            | PianoRollEdit::MoveSelected { document_id, .. }
            | PianoRollEdit::Erase { document_id, .. } => *document_id,
        };

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
        if doc.payload.notation().is_none() {
            self.last_error = Some(OpenError::Invalid {
                path: doc.path.clone().unwrap_or_else(|| PathBuf::from("unsaved")),
                reason: String::from("only notation documents can be saved in this slice"),
            });
            return Task::none();
        }
        let document_id = doc.id;
        if let Some(path) = doc.path.clone() {
            return self.write_notation_document(document_id, path);
        }
        pick_save_file(document_id)
    }

    fn write_notation_document(&mut self, document_id: DocumentId, path: PathBuf) -> Task<Message> {
        let Some(doc) = self.documents.get_mut(document_id) else {
            return Task::none();
        };
        let Some(score) = doc.payload.notation() else {
            return Task::none();
        };
        let json = match score.to_pretty_json() {
            Ok(json) => json,
            Err(error) => {
                self.last_error = Some(OpenError::Invalid {
                    path: path.clone(),
                    reason: error.to_string(),
                });
                return Task::none();
            }
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
        let status = row![text(self.status())].padding(8);

        let mut shell = column![menu_bar::global_menu_bar(), status].spacing(0);

        if self
            .documents
            .active_document()
            .and_then(|doc| doc.payload.notation())
            .is_some()
        {
            shell = shell.push(
                row![piano_roll_tool_bar(&self.piano_roll_ui)]
                    .spacing(6)
                    .padding([0, 8]),
            );
        }

        let tabs = self.tab_bar();
        let body = self.document_panel();

        shell.push(tabs).push(body).into()
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
                    Some(graph) => graph_canvas(doc.id, graph),
                    None => view_error_placeholder(format!(
                        "View `{VIEW_GRAPH_DAG}` requires an Imp graph tip (role `{role}`)"
                    )),
                },
                Some((role, VIEW_PIANO_ROLL)) => match doc.payload.notation() {
                    Some(notation) => piano_roll_panel(doc.id, notation, &self.piano_roll_ui),
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
