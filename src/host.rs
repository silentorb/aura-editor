//! Durable Miro-profile pull workflow: session handle helpers and NodeId → stacked open.
//!
//! Product open is Miro-profile-first (workspace + NodeId). Callers (File → Open…, missing-card
//! resolve, tests) use [`open_pulled_node`] to resolve a NodeId through the Aura host
//! index, pull the Mir blob, and materialize it into [`crate::ModelGraph`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aura_imp::Registry;
use aura_integration::{
    authoring_path_for_mir_record, build_host_node_index, open_aura_session_for_root,
    open_aura_session_in_memory, pull_by_node_id, HostError, HostResolver,
    HostNodeIndex, HostNodeRef, Session, NodeResolver,
};
use aura_interfaces::DocumentOpenError;
use imp_core_types::{Graph, NodeId};
use imp_graph_editor::{CatalogPort, NodePortInfo, RemoteNodeDisplay, RemoteNodeMap};
use thiserror::Error;

use crate::extensions::{ExtensionRuntime, StackedOpen};

/// Host-resolved remote card: display fields plus port templates for the canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedRemote {
    /// Title / type for the card (same presentation as a local node).
    pub display: RemoteNodeDisplay,
    /// Port catalog entry derived from the host-resolved definition.
    pub ports: NodePortInfo,
}

/// Errors from opening a host session or pulling a node into the document stack.
#[derive(Debug, Error)]
pub enum HostOpenError {
    /// Session, index, or projection failure.
    #[error("{0}")]
    Session(#[from] HostError),
    /// Stacked document-type open failed for the pulled bytes.
    #[error("{0}")]
    Document(#[from] DocumentOpenError),
}

impl HostOpenError {
    /// Maps into the shell's existing open-error status type.
    pub fn into_open_error(self) -> DocumentOpenError {
        match self {
            Self::Document(err) => err,
            Self::Session(HostError::MissingNode { node_id }) => DocumentOpenError::Invalid {
                path: PathBuf::from(node_id),
                reason: String::from("no host record indexed for node"),
            },
            Self::Session(HostError::MissingRecord { id }) => DocumentOpenError::Invalid {
                path: PathBuf::from(id),
                reason: String::from("no record in projection"),
            },
            Self::Session(err) => DocumentOpenError::Invalid {
                path: PathBuf::from("host"),
                reason: err.to_string(),
            },
        }
    }
}

/// Result of resolving and pulling a NodeId through the host.
pub struct PulledNodeOpen {
    /// Stacked document tip for the pulled Mir blob.
    pub opened: StackedOpen,
    /// Product NodeId that was requested.
    pub node_id: String,
    /// Resolved Mir blob metadata.
    pub node_ref: HostNodeRef,
}

/// Opens (or replaces) a host session for `content_root` with the host DB path.
pub fn open_session(content_root: impl Into<PathBuf>) -> Result<Session, HostError> {
    open_aura_session_for_root(content_root, true)
}

/// Opens an in-memory host session (tests).
pub fn open_session_in_memory(
    content_root: impl Into<PathBuf>,
) -> Result<Session, HostError> {
    open_aura_session_in_memory(content_root, false)
}

/// Builds the host NodeId index for a content root (text SoT scan).
pub fn host_node_index(content_root: impl AsRef<Path>) -> Result<HostNodeIndex, HostError> {
    build_host_node_index(content_root)
}

/// Polls the session watch, resolves `node_id`, pulls the Mir blob, and runs stacked open.
pub fn open_pulled_node(
    extensions: &ExtensionRuntime,
    session: &mut Session,
    index: &HostNodeIndex,
    node_id: &str,
) -> Result<PulledNodeOpen, HostOpenError> {
    session.poll_watch().map_err(HostError::from)?;
    let record = pull_by_node_id(session, index, node_id)?;
    let node_ref = index
        .get(node_id)
        .cloned()
        .ok_or_else(|| HostError::MissingNode {
            node_id: node_id.to_string(),
        })?;
    let authoring = authoring_path_for_mir_record(session.content_root(), &record.id);
    let opened = extensions.open_document_bytes(&authoring, record.content.as_bytes())?;
    Ok(PulledNodeOpen {
        opened,
        node_id: node_id.to_string(),
        node_ref,
    })
}

/// Convenience: content root currently bound to `session`.
pub fn session_content_root(session: &Session) -> &Path {
    session.content_root()
}

/// Edge-referenced NodeIds absent from the local map but present in the host index.
///
/// Resolves title, type, and ports via [`HostResolver`] so remote cards look
/// like local ones without inserting Imp proxy nodes. Index hits that fail tip
/// resolve still appear as remotes (non-red) with empty display/ports.
pub fn indexed_remote_nodes(
    graph: &Graph,
    content_root: impl AsRef<Path>,
    registry: &Registry,
) -> Result<BTreeMap<NodeId, IndexedRemote>, HostError> {
    let content_root = content_root.as_ref();
    let index = build_host_node_index(content_root)?;
    let resolver = HostResolver {
        content_root,
        index: &index,
        registry,
    };
    let mut remotes = BTreeMap::new();
    for edge in graph.edges.values() {
        for node_id in [&edge.from.node, &edge.to.node] {
            if graph.nodes.contains_key(node_id) || remotes.contains_key(node_id) {
                continue;
            }
            if index.get(node_id).is_none() {
                continue;
            }
            let remote = match resolver.resolve_endpoint(node_id) {
                Some(endpoint) => IndexedRemote {
                    display: RemoteNodeDisplay {
                        title: endpoint.node.title,
                        node_type: endpoint.node.node_type,
                    },
                    ports: NodePortInfo {
                        inputs: endpoint
                            .definition
                            .inputs
                            .values()
                            .map(|port| CatalogPort {
                                id: port.id.clone(),
                                signal_key: serde_json::to_string(&port.signal_type)
                                    .unwrap_or_default(),
                                variadic: port.variadic,
                            })
                            .collect(),
                        outputs: endpoint.definition.outputs.keys().cloned().collect(),
                    },
                },
                None => IndexedRemote {
                    display: RemoteNodeDisplay::default(),
                    ports: NodePortInfo::default(),
                },
            };
            remotes.insert(node_id.clone(), remote);
        }
    }
    Ok(remotes)
}

/// Display map only (keys = remote-ok NodeIds).
pub fn indexed_remote_display(
    remotes: &BTreeMap<NodeId, IndexedRemote>,
) -> RemoteNodeMap {
    remotes
        .iter()
        .map(|(id, remote)| (id.clone(), remote.display.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn open_pulled_node_pulls_graph_bytes_by_node_id() {
        let dir = std::env::temp_dir().join(format!("aura-host-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(
            dir.join("g.json"),
            r#"{"type":"imp-graph","nodes":{"NODEA":{"id":"NODEA","type":"time","inputs":{}}},"edges":{}}"#,
        )
        .expect("write");

        let mut session = open_session_in_memory(&dir).expect("session");
        let index = host_node_index(&dir).expect("index");
        let runtime = ExtensionRuntime::load_bundled();
        let opened = open_pulled_node(&runtime, &mut session, &index, "NODEA").expect("pull");
        assert_eq!(opened.node_ref.mir_record_id, "g.json");
        assert!(opened
            .opened
            .type_chain
            .iter()
            .any(|t| t == aura_interfaces::DOC_TYPE_IMP_GRAPH));
        let _ = fs::remove_dir_all(&dir);
    }
}
