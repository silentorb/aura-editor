//! Notation open, edit, save, and user-state coverage.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aura_editor::{
    apply_edit, semitone_label, App, Bindings, CommandId, ExtensionRuntime, Message,
    PianoRollEdit, NotationTool, PianoRollUiEntry, SnapResolution, UserDir, CMD_DOCUMENT_NEW_NOTATION,
    CMD_DOCUMENT_SAVE, CMD_NOTATION_TOOL_SELECT, DOC_TYPE_AURA_JSON, DOC_TYPE_JSON,
    DOC_TYPE_NOTATION, USER_STATE_BUCKET_NOTATION, USER_STATE_PIANO_ROLL_UI,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("aura-editor-notation-{label}-{nanos}"));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn opens_notation_fixture() {
    let runtime = ExtensionRuntime::load_bundled();
    let opened = runtime
        .open_document(fixture("notation.json"))
        .expect("notation");
    assert_eq!(
        opened.type_chain,
        vec![
            DOC_TYPE_JSON.to_string(),
            DOC_TYPE_AURA_JSON.to_string(),
            DOC_TYPE_NOTATION.to_string()
        ]
    );
    let notation = opened
        .payload
        .downcast_ref::<aura_editor::Notation>()
        .expect("notation tip");
    assert_eq!(notation.events.len(), 2);
    assert_eq!(notation.events[0].semitone, 60);
}

#[test]
fn new_edit_save_reopen_round_trip() {
    let dir = temp_dir("roundtrip");
    let path = dir.join("notation.json");

    let mut app = App::with_bindings(Bindings::empty());
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_NEW_NOTATION)));
    assert_eq!(app.documents().documents().len(), 1);
    let doc = app.documents().active_document().unwrap();
    assert!(doc.path.is_none());
    assert!(doc.dirty);
    let document_id = doc.id;

    let _ = app.update(Message::PianoRoll(PianoRollEdit::Draw {
        document_id,
        semitone: 60,
        start_beats: 0.0,
        duration_beats: 0.0625,
    }));
    assert!(app.documents().active_document().unwrap().dirty);
    assert_eq!(
        app.documents()
            .active_document()
            .unwrap()
            .payload
            .notation()
            .unwrap()
            .events
            .len(),
        1
    );

    let _ = app.update(Message::SaveToPath {
        document_id,
        path: path.clone(),
    });
    assert!(!app.documents().active_document().unwrap().dirty);
    assert_eq!(
        app.documents().active_document().unwrap().path.as_deref(),
        Some(path.as_path())
    );
    assert!(path.is_file());

    let mut reopened = App::with_bindings(Bindings::empty());
    let _ = reopened.update(Message::OpenPath(path.clone()));
    let notation = reopened
        .documents()
        .active_document()
        .unwrap()
        .payload
        .notation()
        .unwrap();
    assert_eq!(notation.events.len(), 1);
    assert_eq!(notation.events[0].semitone, 60);
    assert_eq!(semitone_label(60), "C4");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn tool_commands_update_piano_roll_ui() {
    let mut app = App::with_bindings(Bindings::empty());
    assert_eq!(app.piano_roll_ui().tool, NotationTool::Draw);
    let _ = app.update(Message::Command(CommandId::new(CMD_NOTATION_TOOL_SELECT)));
    assert_eq!(app.piano_roll_ui().tool, NotationTool::Select);
    let _ = app.update(Message::SnapResolutionSelected(SnapResolution::Eighth));
    assert_eq!(app.piano_roll_ui().snap_resolution, SnapResolution::Eighth);
}

#[test]
fn piano_roll_ui_user_state_round_trips() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let user_dir = UserDir::from_path(
        std::env::temp_dir().join(format!("aura-editor-notation-ui-{nanos}")),
    );
    let _ = fs::remove_dir_all(user_dir.path());

    let mut app = App::with_user_dir(
        user_dir.clone(),
        ExtensionRuntime::load_bundled(),
        Bindings::empty(),
    );
    let _ = app.update(Message::Command(CommandId::new(CMD_NOTATION_TOOL_SELECT)));
    let _ = app.update(Message::SnapResolutionSelected(SnapResolution::Quarter));
    app.flush_user_state_now();

    let bucket = fs::read_to_string(user_dir.bucket_path(USER_STATE_BUCKET_NOTATION)).unwrap();
    assert!(bucket.contains(USER_STATE_PIANO_ROLL_UI));
    assert!(bucket.contains("select"));
    assert!(bucket.contains("1/4"));

    let restored = App::with_user_dir(
        user_dir.clone(),
        ExtensionRuntime::load_bundled(),
        Bindings::empty(),
    );
    assert_eq!(restored.piano_roll_ui().tool, NotationTool::Select);
    assert_eq!(restored.piano_roll_ui().snap_resolution, SnapResolution::Quarter);

    let entry = restored
        .user_state()
        .get_entry::<PianoRollUiEntry>(USER_STATE_BUCKET_NOTATION, USER_STATE_PIANO_ROLL_UI)
        .expect("entry");
    assert_eq!(entry.active_tool, "select");

    let _ = fs::remove_dir_all(user_dir.path());
}

#[test]
fn untitled_new_is_not_persisted_in_open_documents() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let user_dir =
        UserDir::from_path(std::env::temp_dir().join(format!("aura-editor-untitled-{nanos}")));
    let _ = fs::remove_dir_all(user_dir.path());

    let mut app = App::with_user_dir(
        user_dir.clone(),
        ExtensionRuntime::load_bundled(),
        Bindings::empty(),
    );
    let _ = app.update(Message::Command(CommandId::new(CMD_DOCUMENT_NEW_NOTATION)));
    app.flush_user_state_now();

    let restored = App::with_user_dir(
        user_dir.clone(),
        ExtensionRuntime::load_bundled(),
        Bindings::empty(),
    );
    assert!(restored.documents().documents().is_empty());

    let _ = fs::remove_dir_all(user_dir.path());
}

#[test]
fn apply_edit_move_updates_note() {
    let mut notation = aura_editor::Notation::empty();
    notation
        .events
        .push(aura_editor::NotationNote::new(60, 0.0, 0.25));
    let changed = apply_edit(
        &mut notation,
        &PianoRollEdit::MoveSelected {
            document_id: aura_editor::DocumentId::from_raw(1),
            note_index: 0,
            start_beats: 1.0,
            semitone: 62,
        },
    );
    assert!(changed);
    assert_eq!(notation.events[0].start_beats, 1.0);
    assert_eq!(notation.events[0].semitone, 62);

    // silence unused save command constant in this module's imports if unused
    let _ = CMD_DOCUMENT_SAVE;
}
