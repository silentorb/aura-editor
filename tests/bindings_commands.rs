//! Command dispatch and key→binding resolution through the app.

use std::path::PathBuf;

use aura_editor::{
    App, Bindings, CommandId, Message, DEFAULT_BINDINGS_JSON,
};
use iced::keyboard::{Key, Modifiers};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn command_close_active_closes_tab() {
    let mut app = App::with_bindings(Bindings::empty());
    let _ = app.update(Message::OpenPath(fixture("sine.json")));
    assert_eq!(app.documents().documents().len(), 1);

    let _ = app.update(Message::Command(CommandId::DocumentCloseActive));
    assert!(app.documents().documents().is_empty());
}

#[test]
fn command_close_active_noop_when_empty() {
    let mut app = App::with_bindings(Bindings::empty());
    let _ = app.update(Message::Command(CommandId::DocumentCloseActive));
    assert!(app.documents().documents().is_empty());
}

#[test]
fn key_input_dispatches_bound_close() {
    let bindings = Bindings::from_json_str(DEFAULT_BINDINGS_JSON).unwrap();
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
    let bindings = Bindings::from_json_str(DEFAULT_BINDINGS_JSON).unwrap();
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
    let _task = app.update(Message::Command(CommandId::DocumentOpen));
}
