//! Open-documents user-state restore against a temporary Aura user directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use aura_editor::{
    App, Bindings, DocumentOrigin, ExtensionRuntime, Message, UserDir, USER_STATE_BUCKET_DOCUMENTS,
    USER_STATE_OPEN_DOCUMENTS,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_user_dir(label: &str) -> UserDir {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("aura-editor-user-state-{label}-{nanos}"));
    let _ = fs::remove_dir_all(&path);
    UserDir::from_path(path)
}

fn app_with_user_dir(user_dir: UserDir) -> App {
    App::with_user_dir(
        user_dir,
        ExtensionRuntime::load_bundled(),
        Bindings::empty(),
    )
}

#[test]
fn restores_open_paths_and_active_tab() {
    let user_dir = temp_user_dir("restore");
    let sine = fixture("sine.json");
    fs::create_dir_all(user_dir.path()).unwrap();
    let second = user_dir.path().join("second.json");
    fs::copy(&sine, &second).unwrap();

    let mut app = app_with_user_dir(user_dir.clone());
    let _ = app.update(Message::OpenPath(sine.clone()));
    let _ = app.update(Message::OpenPath(second.clone()));
    let first_id = app.documents().find_by_path(&sine).expect("sine open").id;
    let _ = app.update(Message::SelectTab(first_id));
    app.flush_user_state_now();

    let restored = app_with_user_dir(user_dir.clone());
    let paths: Vec<&Path> = restored
        .documents()
        .documents()
        .iter()
        .filter_map(|doc| doc.path.as_deref())
        .collect();
    assert_eq!(paths, vec![sine.as_path(), second.as_path()]);
    assert_eq!(
        restored
            .documents()
            .active_document()
            .and_then(|doc| doc.path.as_deref()),
        Some(sine.as_path())
    );

    let bucket = fs::read_to_string(user_dir.bucket_path(USER_STATE_BUCKET_DOCUMENTS)).unwrap();
    assert!(bucket.contains(USER_STATE_OPEN_DOCUMENTS));

    let _ = fs::remove_dir_all(user_dir.path());
}

#[test]
fn missing_path_on_restore_is_skipped() {
    let user_dir = temp_user_dir("missing");
    let sine = fixture("sine.json");
    let missing = user_dir.path().join("gone.json");
    fs::create_dir_all(user_dir.path()).unwrap();

    let mut app = app_with_user_dir(user_dir.clone());
    let _ = app.update(Message::OpenPath(sine.clone()));
    app.flush_user_state_now();

    let bucket_path = user_dir.bucket_path(USER_STATE_BUCKET_DOCUMENTS);
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&bucket_path).unwrap()).unwrap();
    value["entries"]["open-documents"]["openDocuments"] = serde_json::json!([
        { "kind": "filesystem", "path": missing.to_string_lossy() },
        { "kind": "filesystem", "path": sine.to_string_lossy() }
    ]);
    value["entries"]["open-documents"]["active"] =
        serde_json::json!({ "kind": "filesystem", "path": sine.to_string_lossy() });
    value["entries"]["open-documents"]
        .as_object_mut()
        .unwrap()
        .remove("openPaths");
    value["entries"]["open-documents"]
        .as_object_mut()
        .unwrap()
        .remove("activePath");
    fs::write(&bucket_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

    let restored = app_with_user_dir(user_dir.clone());
    assert_eq!(restored.documents().documents().len(), 1);
    assert_eq!(
        restored
            .documents()
            .active_document()
            .and_then(|doc| doc.path.as_deref()),
        Some(sine.as_path())
    );

    let _ = fs::remove_dir_all(user_dir.path());
}

#[test]
fn restores_miro_workspace_and_node_tabs() {
    let user_dir = temp_user_dir("miro-restore");
    fs::create_dir_all(user_dir.path()).unwrap();
    let workspace = user_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    fs::copy(fixture("sine.json"), workspace.join("sine.json")).unwrap();

    const SINE_OSC_NODE: &str = "0033AFBGAY03CEW3Z5AP3J086B";

    {
        let mut app = app_with_user_dir(user_dir.clone());
        // Disk-backed workspace so restore can reopen the same content root.
        app.open_miro_workspace(&workspace).expect("workspace");
        let _ = app.update(Message::OpenMiroNodeId(SINE_OSC_NODE.into()));
        app.flush_user_state_now();
        // Drop the session before restore so the SQLite projection is not locked.
    }

    let restored = app_with_user_dir(user_dir.clone());
    assert_eq!(restored.documents().documents().len(), 1);
    assert_eq!(
        restored.documents().active_document().and_then(|doc| {
            restored.documents().origin(doc.id).cloned()
        }),
        Some(DocumentOrigin::Miro {
            content_root: workspace.clone(),
            node_id: SINE_OSC_NODE.into(),
            mir_record_id: "sine.json".into(),
        })
    );
    assert!(restored.miro_session().is_some());

    let _ = fs::remove_dir_all(user_dir.path());
}
