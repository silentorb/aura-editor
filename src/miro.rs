//! Durable Miro pull workflow: session handle helpers and record → stacked open.
//!
//! Product File → Open remains filesystem-only. Callers (tests, later UI / node-ref
//! navigation) use [`open_miro_record`] to materialize projection records into the
//! stacked document-type pipeline and then into [`crate::ModelGraph`].

use std::path::{Path, PathBuf};

use aura_integration::{
    open_aura_miro_session_for_root, open_aura_miro_session_in_memory, AuraMiroError, MiroSession,
};
use aura_interfaces::DocumentOpenError;
use thiserror::Error;

use crate::document::authoring_path_for_record;
use crate::extensions::{ExtensionRuntime, StackedOpen};

/// Errors from opening a Miro session or pulling a record into the document stack.
#[derive(Debug, Error)]
pub enum MiroOpenError {
    /// Session or projection failure.
    #[error("{0}")]
    Session(#[from] AuraMiroError),
    /// Stacked document-type open failed for the pulled bytes.
    #[error("{0}")]
    Document(#[from] DocumentOpenError),
}

impl MiroOpenError {
    /// Maps into the shell's existing open-error status type.
    pub fn into_open_error(self) -> DocumentOpenError {
        match self {
            Self::Document(err) => err,
            Self::Session(AuraMiroError::MissingRecord { id }) => DocumentOpenError::Invalid {
                path: PathBuf::from(id),
                reason: String::from("no record in Miro projection"),
            },
            Self::Session(err) => DocumentOpenError::Invalid {
                path: PathBuf::from("miro"),
                reason: err.to_string(),
            },
        }
    }
}

/// Opens (or replaces) a Miro session for `content_root` with the host DB path.
pub fn open_miro_session(content_root: impl Into<PathBuf>) -> Result<MiroSession, AuraMiroError> {
    open_aura_miro_session_for_root(content_root, true)
}

/// Opens an in-memory Miro session (tests).
pub fn open_miro_session_in_memory(
    content_root: impl Into<PathBuf>,
) -> Result<MiroSession, AuraMiroError> {
    open_aura_miro_session_in_memory(content_root, false)
}

/// Polls the session watch, pulls `record_id`, and runs the stacked document open.
pub fn open_miro_record(
    extensions: &ExtensionRuntime,
    session: &mut MiroSession,
    record_id: &str,
) -> Result<StackedOpen, MiroOpenError> {
    session
        .poll_watch()
        .map_err(AuraMiroError::from)?;
    let record = session
        .pull_record(record_id)
        .map_err(AuraMiroError::from)?
        .ok_or_else(|| AuraMiroError::MissingRecord {
            id: record_id.to_string(),
        })?;
    let authoring = authoring_path_for_record(session.content_root(), &record.id);
    Ok(extensions.open_document_bytes(&authoring, record.content.as_bytes())?)
}

/// Convenience: content root currently bound to `session`.
pub fn session_content_root(session: &MiroSession) -> &Path {
    session.content_root()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn open_miro_record_pulls_graph_bytes() {
        let dir = std::env::temp_dir().join(format!("aura-miro-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(
            dir.join("g.json"),
            r#"{"type":"imp-graph","nodes":{"n":{"id":"n","type":"time","inputs":{}}},"edges":{}}"#,
        )
        .expect("write");

        let mut session = open_miro_session_in_memory(&dir).expect("session");
        let runtime = ExtensionRuntime::load_bundled();
        let opened = open_miro_record(&runtime, &mut session, "g.json").expect("pull");
        assert!(opened
            .type_chain
            .iter()
            .any(|t| t == aura_interfaces::DOC_TYPE_IMP_GRAPH));
        let _ = fs::remove_dir_all(&dir);
    }
}
