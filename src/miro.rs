//! Durable Miro pull workflow: session handle helpers and NodeId → stacked open.
//!
//! Product open is Miro-first (workspace + NodeId). Callers (File → Open…, missing-card
//! resolve, tests) use [`open_miro_node`] to resolve a NodeId through the Aura host
//! index, pull the Mir blob, and materialize it into [`crate::ModelGraph`].

use std::path::{Path, PathBuf};

use std::collections::BTreeSet;

use aura_integration::{
    authoring_path_for_mir_record, build_miro_node_index, open_aura_miro_session_for_root,
    open_aura_miro_session_in_memory, pull_by_node_id, AuraMiroError, EXTERNAL_SCORE_NODE_TYPE,
    MiroNodeIndex, MiroNodeRef, MiroSession,
};
use aura_interfaces::DocumentOpenError;
use imp_core_types::{Graph, Node};
use thiserror::Error;

use crate::extensions::{ExtensionRuntime, StackedOpen};

/// Errors from opening a Miro session or pulling a node into the document stack.
#[derive(Debug, Error)]
pub enum MiroOpenError {
    /// Session, index, or projection failure.
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
            Self::Session(AuraMiroError::MissingNode { node_id }) => DocumentOpenError::Invalid {
                path: PathBuf::from(node_id),
                reason: String::from("no Miro record indexed for node"),
            },
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

/// Result of resolving and pulling a NodeId through Miro.
pub struct MiroNodeOpen {
    /// Stacked document tip for the pulled Mir blob.
    pub opened: StackedOpen,
    /// Product NodeId that was requested.
    pub node_id: String,
    /// Resolved Mir blob metadata.
    pub node_ref: MiroNodeRef,
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

/// Builds the host NodeId index for a content root (text SoT scan).
pub fn miro_node_index(content_root: impl AsRef<Path>) -> Result<MiroNodeIndex, AuraMiroError> {
    build_miro_node_index(content_root)
}

/// Polls the session watch, resolves `node_id`, pulls the Mir blob, and runs stacked open.
pub fn open_miro_node(
    extensions: &ExtensionRuntime,
    session: &mut MiroSession,
    index: &MiroNodeIndex,
    node_id: &str,
) -> Result<MiroNodeOpen, MiroOpenError> {
    session.poll_watch().map_err(AuraMiroError::from)?;
    let record = pull_by_node_id(session, index, node_id)?;
    let node_ref = index
        .get(node_id)
        .cloned()
        .ok_or_else(|| AuraMiroError::MissingNode {
            node_id: node_id.to_string(),
        })?;
    let authoring = authoring_path_for_mir_record(session.content_root(), &record.id);
    let opened = extensions.open_document_bytes(&authoring, record.content.as_bytes())?;
    Ok(MiroNodeOpen {
        opened,
        node_id: node_id.to_string(),
        node_ref,
    })
}

/// Convenience: content root currently bound to `session`.
pub fn session_content_root(session: &MiroSession) -> &Path {
    session.content_root()
}

/// Insert nullary `external_score` proxies for edge-referenced notation tip NodeIds.
///
/// Scans `content_root` with the host Miro index (no session required). Endpoints that
/// already exist in `graph.nodes`, or that index as non-`notation` tips, are left alone
/// so red missing cards still apply to true unknowns. Does not mark documents dirty —
/// callers use this at open time to match headless translate hydration.
pub fn hydrate_notation_tip_refs(
    graph: &mut Graph,
    content_root: impl AsRef<Path>,
) -> Result<usize, AuraMiroError> {
    let index = build_miro_node_index(content_root)?;
    let mut candidates = BTreeSet::new();
    for edge in graph.edges.values() {
        candidates.insert(edge.from.node.clone());
        candidates.insert(edge.to.node.clone());
    }

    let mut inserted = 0;
    for node_id in candidates {
        if graph.nodes.contains_key(&node_id) {
            continue;
        }
        let Some(node_ref) = index.get(&node_id) else {
            continue;
        };
        if node_ref.kind != "notation" {
            continue;
        }
        graph.nodes.insert(
            node_id.clone(),
            Node {
                id: node_id,
                title: None,
                node_type: EXTERNAL_SCORE_NODE_TYPE.to_string(),
                type_args: Vec::new(),
                inputs: Default::default(),
            },
        );
        inserted += 1;
    }
    Ok(inserted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn open_miro_node_pulls_graph_bytes_by_node_id() {
        let dir = std::env::temp_dir().join(format!("aura-miro-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(
            dir.join("g.json"),
            r#"{"type":"imp-graph","nodes":{"NODEA":{"id":"NODEA","type":"time","inputs":{}}},"edges":{}}"#,
        )
        .expect("write");

        let mut session = open_miro_session_in_memory(&dir).expect("session");
        let index = miro_node_index(&dir).expect("index");
        let runtime = ExtensionRuntime::load_bundled();
        let opened = open_miro_node(&runtime, &mut session, &index, "NODEA").expect("pull");
        assert_eq!(opened.node_ref.mir_record_id, "g.json");
        assert!(opened
            .opened
            .type_chain
            .iter()
            .any(|t| t == aura_interfaces::DOC_TYPE_IMP_GRAPH));
        let _ = fs::remove_dir_all(&dir);
    }
}
