//! Command dispatch and key→binding resolution through the app.

use std::collections::HashSet;
use std::path::PathBuf;

use aura_editor::{
    App, Bindings, CommandId, ExtensionRuntime, Message, CMD_DOCUMENT_CLOSE_ACTIVE,
    CMD_DOCUMENT_OPEN, DEFAULT_BINDINGS_JSON,
};
use iced::keyboard::{Key, Modifiers};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn known_commands() -> HashSet<String> {
    ExtensionRuntime::load_bundled().command_ids().clone()
}

#[test]
fn command_close_active_closes_tab() {
    let mut app = App::with_bindings(Bindings::empty());
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    assert_eq!(app.documents().documents().len(), 1);

    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_CLOSE_ACTIVE)));
    assert!(app.documents().documents().is_empty());
}

#[test]
fn command_close_active_noop_when_empty() {
    let mut app = App::with_bindings(Bindings::empty());
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_CLOSE_ACTIVE)));
    assert!(app.documents().documents().is_empty());
}

#[test]
fn key_input_dispatches_bound_close() {
    let bindings =
        Bindings::from_json_str(DEFAULT_BINDINGS_JSON, &known_commands()).unwrap();
    let mut app = App::with_bindings(bindings);
    let _ = app.update(Message::OpenPath(fixture("sine.json")));

    let _ = app.update(Message::KeyInput {
        key: Key::Character("w".into()),
        modifiers: Modifiers::CTRL,
    });
    assert!(app.documents().documents().is_empty());
}

#[test]
fn unbound_key_is_ignored() {
    let bindings =
        Bindings::from_json_str(DEFAULT_BINDINGS_JSON, &known_commands()).unwrap();
    let mut app = App::with_bindings(bindings);
    let _ = app.update(Message::OpenPath(fixture("sine.json")));

    let _ = app.update(Message::KeyInput {
        key: Key::Character("x".into()),
        modifiers: Modifiers::CTRL,
    });
    assert_eq!(app.documents().documents().len(), 1);
}

#[test]
fn command_open_returns_dialog_task() {
    let mut app = App::with_bindings(Bindings::empty());
    // Opening the dialog is async; we only assert the task is produced (non-panic).
    let _task = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_OPEN)));
}

#[test]
fn disabled_command_is_ignored() {
    let json = r#"{
      "version": 1,
      "extensions": [{ "id": "aura-builtin", "enabled": true }],
      "components": [
        {
          "id": "document.close_active",
          "extensionId": "aura-builtin",
          "kind": "command",
          "implementationId": "document.close_active",
          "enabled": false
        }
      ]
    }"#;
    let extensions = ExtensionRuntime::load_from_json(
        json,
        aura_editor::register_linked_extensions,
    );
    let mut app = App::with_extensions(extensions, Bindings::empty());
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_CLOSE_ACTIVE)));
    assert_eq!(app.documents().documents().len(), 1);
}

#[test]
fn missing_view_role_shows_placeholder() {
    let mut app =
        App::with_extensions(ExtensionRuntime::empty(), Bindings::empty());
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    assert_eq!(app.documents().documents().len(), 1);

    let mut ui = iced_test::simulator(app.view());
    assert!(ui
        .find("No enabled view for role `imp-graph`")
        .is_ok());
}
