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

pub use bindings::{user_bindings_path, Bindings, DEFAULT_BINDINGS_JSON};
pub use command::CommandId;
pub use document::{
    DocumentId, DocumentPayload, DocumentStore, GraphDocument, SessionDocument,
};
pub use extensions::{
    register_linked_extensions, ExtensionRuntime, RegistrationHost, StackedOpen,
    DEFAULT_EXTENSIONS_JSON,
};
pub use graph_file::OpenError;
pub use graph_layout::{layout_graph, GraphLayout, LaidOutNode};
pub use graph_view::{
    center_translation, clamp_scale, pan_by, zoom_at_point, GraphView,
};
pub use logging::init_logging;

pub use aura_interfaces::{
    CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_OPEN, DOC_TYPE_AURA_JSON, DOC_TYPE_IMP_GRAPH,
    DOC_TYPE_IMP_GRAPH_LIBRARY, DOC_TYPE_JSON, ROLE_IMP_GRAPH, ROLE_JSON, VIEW_GRAPH_DAG,
};

use std::path::PathBuf;

use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Fill, Subscription, Task};

use crate::graph_view::graph_canvas;

/// Root application state for the editor shell.
#[derive(Debug, Clone)]
pub struct App {
    documents: DocumentStore,
    /// Last open/load error shown in the status banner.
    last_error: Option<OpenError>,
    /// Resolved input bindings (may be empty).
    bindings: Bindings,
    /// Extension command/view/document-type registries.
    extensions: ExtensionRuntime,
}

/// Messages produced by the multi-document shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// User chose Open… from the toolbar.
    OpenClicked,
    /// Open the graph at this path (from the dialog or tests).
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
}

impl Default for App {
    fn default() -> Self {
        Self::with_bindings(Bindings::empty())
    }
}

impl App {
    /// Creates the initial application state with bundled extensions and bindings.
    pub fn new() -> Self {
        let extensions = ExtensionRuntime::load_bundled();
        let bindings = Bindings::load(extensions.command_ids());
        Self::with_extensions(extensions, bindings)
    }

    /// Creates an app with bundled extensions and an explicit bindings table.
    pub fn with_bindings(bindings: Bindings) -> Self {
        Self::with_extensions(ExtensionRuntime::load_bundled(), bindings)
    }

    /// Creates an app with explicit extension registries and bindings (tests).
    pub fn with_extensions(extensions: ExtensionRuntime, bindings: Bindings) -> Self {
        Self {
            documents: DocumentStore::default(),
            last_error: None,
            bindings,
            extensions,
        }
    }

    /// Window title shown by the native shell.
    pub fn title(&self) -> String {
        match self.documents.active_document() {
            Some(doc) => format!("Aura Editor — {}", doc.title),
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
            String::from("Open a graph file to begin")
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
            Message::OpenClicked => return pick_graph_file(),
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
                    }
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
            }
            Message::CloseTab(id) => {
                self.documents.close(id);
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
                    CMD_DOCUMENT_OPEN => return pick_graph_file(),
                    CMD_DOCUMENT_CLOSE_ACTIVE => {
                        if let Some(id) = self.documents.active_id() {
                            self.documents.close(id);
                        }
                    }
                    _ => {}
                }
            }
        }
        Task::none()
    }

    /// Builds the multi-document shell view.
    pub fn view(&self) -> Element<'_, Message> {
        let toolbar = row![
            button("Open…").on_press(Message::OpenClicked),
            Space::new().width(12),
            text(self.status()),
        ]
        .spacing(8)
        .padding(8);

        let tabs = self.tab_bar();
        let body = self.document_panel();

        column![toolbar, tabs, body].spacing(0).into()
    }

    fn tab_bar(&self) -> Element<'_, Message> {
        if self.documents.documents().is_empty() {
            return Space::new().height(0).into();
        }

        let mut tabs = row![].spacing(4).padding([0, 8]);
        for doc in self.documents.documents() {
            let active = self.documents.active_id() == Some(doc.id);
            let label = if active {
                format!("[{}]", doc.title)
            } else {
                doc.title.clone()
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
            container(text("Open a graph file to begin"))
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

fn view_error_placeholder(message: String) -> Element<'static, Message> {
    container(text(message))
        .center_x(Fill)
        .center_y(Fill)
        .width(Fill)
        .height(Fill)
        .into()
}

fn pick_graph_file() -> Task<Message> {
    Task::perform(pick_graph_file_async(), |path| match path {
        Some(path) => Message::OpenPath(path),
        None => Message::OpenCancelled,
    })
}

async fn pick_graph_file_async() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("Imp graph", &["json"])
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}
