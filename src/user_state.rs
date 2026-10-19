//! Bucketed ephemeral user-state persistence (JSON snapshots under the Aura user dir).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tracing::warn;

use crate::extensions::UserStateBinding;

/// Default subdirectory name under the process working directory.
pub const DEFAULT_USER_DIR_NAME: &str = ".aura-user";

/// Environment variable overriding the Aura user directory.
pub const USER_DIR_ENV: &str = "AURA_USER_DIR";

/// Supported bucket document schema version.
pub const BUCKET_SCHEMA_VERSION: u32 = 1;

/// Debounce idle period before writing dirty buckets.
pub const DEBOUNCE: Duration = Duration::from_millis(500);

/// Failure while reading or writing user-state buckets.
#[derive(Debug, Error)]
pub enum UserStateError {
    /// Filesystem error.
    #[error("user-state I/O error at {path}: {source}")]
    Io {
        /// Path involved in the failure.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// JSON encode failure.
    #[error("user-state JSON encode error: {0}")]
    Encode(String),
}

/// Resolved Aura user directory root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDir {
    path: PathBuf,
}

impl UserDir {
    /// Resolves from `AURA_USER_DIR` when set, otherwise `{cwd}/.aura-user`.
    pub fn resolve() -> Self {
        if let Ok(override_path) = std::env::var(USER_DIR_ENV) {
            let path = PathBuf::from(override_path);
            if !path.as_os_str().is_empty() {
                return Self { path };
            }
        }
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            path: cwd.join(DEFAULT_USER_DIR_NAME),
        }
    }

    /// Explicit directory (tests and tooling).
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Directory path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path of the JSON file for `bucket`.
    pub fn bucket_path(&self, bucket: &str) -> PathBuf {
        self.path.join(format!("{bucket}.json"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct BucketDocument {
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, Value>,
}

impl Default for BucketDocument {
    fn default() -> Self {
        Self {
            version: BUCKET_SCHEMA_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

/// In-memory user-state store with optional on-disk bucket persistence.
#[derive(Debug, Clone)]
pub struct UserStateStore {
    user_dir: Option<UserDir>,
    buckets: HashMap<String, BucketDocument>,
    dirty: HashSet<String>,
    generation: u64,
}

impl Default for UserStateStore {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl UserStateStore {
    /// Store that never reads or writes the filesystem (tests / headless helpers).
    pub fn in_memory() -> Self {
        Self {
            user_dir: None,
            buckets: HashMap::new(),
            dirty: HashSet::new(),
            generation: 0,
        }
    }

    /// Store rooted at `user_dir` (loads happen via [`Self::load_registered`]).
    pub fn open(user_dir: UserDir) -> Self {
        Self {
            user_dir: Some(user_dir),
            buckets: HashMap::new(),
            dirty: HashSet::new(),
            generation: 0,
        }
    }

    /// Whether this store writes buckets to disk.
    pub fn persists_to_disk(&self) -> bool {
        self.user_dir.is_some()
    }

    /// User directory when persistence is enabled.
    pub fn user_dir(&self) -> Option<&UserDir> {
        self.user_dir.as_ref()
    }

    /// Monotonic generation bumped on each mutating `set_entry`.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether any bucket is dirty.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Load bucket files for the registered bindings (missing/invalid → empty).
    pub fn load_registered(&mut self, bindings: &[UserStateBinding]) {
        let Some(user_dir) = self.user_dir.clone() else {
            return;
        };
        let mut buckets: HashSet<&str> = HashSet::new();
        for binding in bindings {
            buckets.insert(binding.bucket.as_str());
        }
        for bucket in buckets {
            let path = user_dir.bucket_path(bucket);
            let document = load_bucket_file(&path);
            self.buckets.insert(bucket.to_string(), document);
        }
    }

    /// Deserializes an entry when present and valid.
    pub fn get_entry<T: DeserializeOwned>(&self, bucket: &str, entry_id: &str) -> Option<T> {
        let value = self.buckets.get(bucket)?.entries.get(entry_id)?;
        match serde_json::from_value(value.clone()) {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                warn!(
                    bucket,
                    entry = entry_id,
                    %error,
                    "ignoring invalid user-state entry"
                );
                None
            }
        }
    }

    /// Updates an entry in memory, marks the bucket dirty, and bumps generation.
    ///
    /// Returns the new generation (for debounced flush tokens).
    pub fn set_entry<T: Serialize>(
        &mut self,
        bucket: &str,
        entry_id: &str,
        value: &T,
    ) -> Result<u64, UserStateError> {
        let encoded = serde_json::to_value(value)
            .map_err(|error| UserStateError::Encode(error.to_string()))?;
        let document = self.buckets.entry(bucket.to_string()).or_default();
        if document.entries.get(entry_id) == Some(&encoded) {
            return Ok(self.generation);
        }
        document.version = BUCKET_SCHEMA_VERSION;
        document.entries.insert(entry_id.to_string(), encoded);
        self.dirty.insert(bucket.to_string());
        self.generation = self.generation.wrapping_add(1);
        Ok(self.generation)
    }

    /// Writes all dirty buckets immediately.
    pub fn flush_now(&mut self) -> Result<(), UserStateError> {
        let Some(user_dir) = self.user_dir.clone() else {
            self.dirty.clear();
            return Ok(());
        };
        if self.dirty.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(user_dir.path()).map_err(|source| UserStateError::Io {
            path: user_dir.path().to_path_buf(),
            source,
        })?;
        let dirty: Vec<String> = self.dirty.iter().cloned().collect();
        for bucket in dirty {
            let document = self.buckets.get(&bucket).cloned().unwrap_or_default();
            let path = user_dir.bucket_path(&bucket);
            write_bucket_file(&path, &document)?;
            self.dirty.remove(&bucket);
        }
        Ok(())
    }

    /// Flushes when `generation` still matches (debounce coalescing).
    pub fn flush_if_generation(&mut self, generation: u64) -> Result<(), UserStateError> {
        if generation != self.generation {
            return Ok(());
        }
        self.flush_now()
    }
}

/// Open-document tab snapshot stored under the `documents` bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenDocumentsEntry {
    /// Open document paths in tab order.
    #[serde(default)]
    pub open_paths: Vec<PathBuf>,
    /// Active document path when one is selected.
    #[serde(default)]
    pub active_path: Option<PathBuf>,
}

/// Writes `data` via a temporary sibling file, then renames over `path`.
pub fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp_path = atomic_temp_path(path);
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

fn load_bucket_file(path: &Path) -> BucketDocument {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<BucketDocument>(&text) {
            Ok(document) if document.version == BUCKET_SCHEMA_VERSION => document,
            Ok(document) => {
                warn!(
                    path = %path.display(),
                    found = document.version,
                    expected = BUCKET_SCHEMA_VERSION,
                    "unsupported user-state bucket version; using empty"
                );
                BucketDocument::default()
            }
            Err(error) => {
                warn!(
                    path = %path.display(),
                    %error,
                    "invalid user-state bucket JSON; using empty"
                );
                BucketDocument::default()
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => BucketDocument::default(),
        Err(error) => {
            warn!(
                path = %path.display(),
                %error,
                "failed to read user-state bucket; using empty"
            );
            BucketDocument::default()
        }
    }
}

fn write_bucket_file(path: &Path, document: &BucketDocument) -> Result<(), UserStateError> {
    let json = serde_json::to_vec_pretty(document)
        .map_err(|error| UserStateError::Encode(error.to_string()))?;
    write_atomic(path, &json).map_err(|source| UserStateError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_user_dir(label: &str) -> UserDir {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("aura-user-state-{label}-{nanos}"));
        let _ = fs::remove_dir_all(&path);
        UserDir::from_path(path)
    }

    #[test]
    fn atomic_replace_round_trips() {
        let dir = temp_user_dir("atomic");
        fs::create_dir_all(dir.path()).unwrap();
        let path = dir.bucket_path("documents");
        write_atomic(&path, br#"{"version":1,"entries":{}}"#).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"version":1,"entries":{}}"#
        );
        write_atomic(&path, br#"{"version":1,"entries":{"a":1}}"#).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"version":1,"entries":{"a":1}}"#
        );
        let _ = fs::remove_dir_all(dir.path());
    }

    #[test]
    fn flush_now_writes_dirty_buckets() {
        let dir = temp_user_dir("flush");
        let mut store = UserStateStore::open(dir.clone());
        let entry = OpenDocumentsEntry {
            open_paths: vec![PathBuf::from("/tmp/a.json")],
            active_path: Some(PathBuf::from("/tmp/a.json")),
        };
        let gen = store
            .set_entry("documents", "open-documents", &entry)
            .unwrap();
        assert!(gen > 0);
        assert!(store.is_dirty());
        store.flush_now().unwrap();
        assert!(!store.is_dirty());

        let mut reloaded = UserStateStore::open(dir.clone());
        reloaded.load_registered(&[UserStateBinding {
            entry_id: "open-documents".into(),
            bucket: "documents".into(),
        }]);
        let loaded: OpenDocumentsEntry = reloaded
            .get_entry("documents", "open-documents")
            .expect("entry");
        assert_eq!(loaded, entry);
        let _ = fs::remove_dir_all(dir.path());
    }

    #[test]
    fn flush_if_generation_coalesces() {
        let dir = temp_user_dir("coalesce");
        let mut store = UserStateStore::open(dir.clone());
        let first = store
            .set_entry(
                "documents",
                "open-documents",
                &OpenDocumentsEntry {
                    open_paths: vec![PathBuf::from("/a")],
                    active_path: None,
                },
            )
            .unwrap();
        let second = store
            .set_entry(
                "documents",
                "open-documents",
                &OpenDocumentsEntry {
                    open_paths: vec![PathBuf::from("/b")],
                    active_path: None,
                },
            )
            .unwrap();
        assert_ne!(first, second);
        store.flush_if_generation(first).unwrap();
        assert!(store.is_dirty());
        store.flush_if_generation(second).unwrap();
        assert!(!store.is_dirty());
        let text = fs::read_to_string(dir.bucket_path("documents")).unwrap();
        assert!(text.contains("/b"));
        assert!(!text.contains("\"/a\""));
        let _ = fs::remove_dir_all(dir.path());
    }

    #[test]
    fn missing_and_invalid_buckets_load_empty() {
        let dir = temp_user_dir("empty");
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(dir.bucket_path("documents"), "not-json").unwrap();
        let mut store = UserStateStore::open(dir.clone());
        store.load_registered(&[UserStateBinding {
            entry_id: "open-documents".into(),
            bucket: "documents".into(),
        }]);
        let entry: Option<OpenDocumentsEntry> = store.get_entry("documents", "open-documents");
        assert!(entry.is_none());
        let _ = fs::remove_dir_all(dir.path());
    }

    #[test]
    fn multi_entry_same_bucket() {
        let dir = temp_user_dir("multi");
        let mut store = UserStateStore::open(dir.clone());
        store
            .set_entry("shared", "one", &serde_json::json!({"n": 1}))
            .unwrap();
        store
            .set_entry("shared", "two", &serde_json::json!({"n": 2}))
            .unwrap();
        store.flush_now().unwrap();
        let mut reloaded = UserStateStore::open(dir.clone());
        reloaded.load_registered(&[
            UserStateBinding {
                entry_id: "one".into(),
                bucket: "shared".into(),
            },
            UserStateBinding {
                entry_id: "two".into(),
                bucket: "shared".into(),
            },
        ]);
        let one: Value = reloaded.get_entry("shared", "one").unwrap();
        let two: Value = reloaded.get_entry("shared", "two").unwrap();
        assert_eq!(one.get("n"), Some(&Value::from(1)));
        assert_eq!(two.get("n"), Some(&Value::from(2)));
        let _ = fs::remove_dir_all(dir.path());
    }

    #[test]
    fn in_memory_store_does_not_require_disk() {
        let mut store = UserStateStore::in_memory();
        assert!(!store.persists_to_disk());
        store
            .set_entry(
                "documents",
                "open-documents",
                &OpenDocumentsEntry::default(),
            )
            .unwrap();
        store.flush_now().unwrap();
    }
}
