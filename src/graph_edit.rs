//! Pure Imp graph mutation helpers for the editor.

use std::collections::{BTreeMap, HashSet};

use aura_imp::{get_node_type, Registry};
use imp_core_types::{Edge, Graph, Node, NodeId, PortReference};
use thiserror::Error;

/// Errors from graph edit helpers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphEditError {
    /// Node id already exists.
    #[error("node `{0}` already exists")]
    NodeExists(String),
    /// Node id was not found.
    #[error("node `{0}` not found")]
    NodeMissing(String),
    /// Edge id was not found.
    #[error("edge `{0}` not found")]
    EdgeMissing(String),
    /// Rename target collides with an existing id.
    #[error("cannot rename to `{0}`; id already exists")]
    RenameCollision(String),
    /// Empty id is invalid.
    #[error("node id must not be empty")]
    EmptyId,
    /// No compatible free port pair for a simple connection.
    #[error("no compatible free ports between `{from}` and `{to}`")]
    NoPortPair {
        /// Source node.
        from: String,
        /// Target node.
        to: String,
    },
    /// Node type is unknown in the registry (precise connect / auto-pair).
    #[error("unknown node type `{0}`")]
    UnknownType(String),
}

/// Allocates a unique node id derived from `base` (`sine`, `sine_2`, …).
pub fn unique_node_id(graph: &Graph, base: &str) -> NodeId {
    let base = if base.trim().is_empty() { "node" } else { base };
    if !graph.nodes.contains_key(base) {
        return base.to_string();
    }
    let mut index = 2u32;
    loop {
        let candidate = format!("{base}_{index}");
        if !graph.nodes.contains_key(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// Inserts a node with empty inputs.
pub fn add_node(
    graph: &mut Graph,
    id: impl Into<NodeId>,
    node_type: impl Into<String>,
) -> Result<NodeId, GraphEditError> {
    let id = id.into();
    if id.trim().is_empty() {
        return Err(GraphEditError::EmptyId);
    }
    if graph.nodes.contains_key(&id) {
        return Err(GraphEditError::NodeExists(id));
    }
    let node_type = node_type.into();
    graph.nodes.insert(
        id.clone(),
        Node {
            id: id.clone(),
            node_type,
            type_args: Vec::new(),
            inputs: BTreeMap::new(),
        },
    );
    Ok(id)
}

/// Removes a node and all incident edges.
pub fn remove_node(graph: &mut Graph, node_id: &str) -> Result<(), GraphEditError> {
    if graph.nodes.remove(node_id).is_none() {
        return Err(GraphEditError::NodeMissing(node_id.to_string()));
    }
    graph.edges.retain(|_, edge| {
        edge.from.node != node_id && edge.to.node != node_id
    });
    Ok(())
}

/// Renames a node and rewrites edge endpoints that reference it.
pub fn rename_node(
    graph: &mut Graph,
    old_id: &str,
    new_id: impl Into<NodeId>,
) -> Result<NodeId, GraphEditError> {
    let new_id = new_id.into();
    if new_id.trim().is_empty() {
        return Err(GraphEditError::EmptyId);
    }
    if old_id == new_id {
        return Ok(new_id);
    }
    if graph.nodes.contains_key(&new_id) {
        return Err(GraphEditError::RenameCollision(new_id));
    }
    let Some(mut node) = graph.nodes.remove(old_id) else {
        return Err(GraphEditError::NodeMissing(old_id.to_string()));
    };
    node.id = new_id.clone();
    graph.nodes.insert(new_id.clone(), node);
    for edge in graph.edges.values_mut() {
        if edge.from.node == old_id {
            edge.from.node = new_id.clone();
        }
        if edge.to.node == old_id {
            edge.to.node = new_id.clone();
        }
    }
    Ok(new_id)
}

/// Changes the type of an existing node (inputs cleared).
pub fn set_node_type(
    graph: &mut Graph,
    node_id: &str,
    node_type: impl Into<String>,
) -> Result<(), GraphEditError> {
    let Some(node) = graph.nodes.get_mut(node_id) else {
        return Err(GraphEditError::NodeMissing(node_id.to_string()));
    };
    node.node_type = node_type.into();
    node.type_args.clear();
    node.inputs.clear();
    Ok(())
}

/// Allocates a unique edge id.
pub fn unique_edge_id(graph: &Graph, from: &str, from_port: &str, to: &str, to_port: &str) -> String {
    let base = format!("{from}_{from_port}_to_{to}_{to_port}");
    if !graph.edges.contains_key(&base) {
        return base;
    }
    let mut index = 2u32;
    loop {
        let candidate = format!("{base}_{index}");
        if !graph.edges.contains_key(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// Inserts a port-level edge.
pub fn add_edge(
    graph: &mut Graph,
    from_node: impl Into<NodeId>,
    from_port: impl Into<String>,
    to_node: impl Into<NodeId>,
    to_port: impl Into<String>,
) -> Result<String, GraphEditError> {
    let from_node = from_node.into();
    let to_node = to_node.into();
    let from_port = from_port.into();
    let to_port = to_port.into();
    if !graph.nodes.contains_key(&from_node) {
        return Err(GraphEditError::NodeMissing(from_node));
    }
    if !graph.nodes.contains_key(&to_node) {
        return Err(GraphEditError::NodeMissing(to_node));
    }
    let id = unique_edge_id(graph, &from_node, &from_port, &to_node, &to_port);
    graph.edges.insert(
        id.clone(),
        Edge {
            from: PortReference {
                node: from_node,
                port: from_port,
            },
            to: PortReference {
                node: to_node,
                port: to_port,
            },
        },
    );
    Ok(id)
}

/// Removes one edge by id.
pub fn remove_edge(graph: &mut Graph, edge_id: &str) -> Result<(), GraphEditError> {
    if graph.edges.remove(edge_id).is_none() {
        return Err(GraphEditError::EdgeMissing(edge_id.to_string()));
    }
    Ok(())
}

/// Removes every edge between two nodes (either direction).
pub fn remove_edges_between(graph: &mut Graph, a: &str, b: &str) -> usize {
    let before = graph.edges.len();
    graph.edges.retain(|_, edge| {
        let ab = edge.from.node == a && edge.to.node == b;
        let ba = edge.from.node == b && edge.to.node == a;
        !(ab || ba)
    });
    before - graph.edges.len()
}

/// Picks an unused compatible output→input port pair for a simple connection.
pub fn auto_pair_ports(
    registry: &Registry,
    graph: &Graph,
    from_node: &str,
    to_node: &str,
) -> Result<(String, String), GraphEditError> {
    let from = graph
        .nodes
        .get(from_node)
        .ok_or_else(|| GraphEditError::NodeMissing(from_node.to_string()))?;
    let to = graph
        .nodes
        .get(to_node)
        .ok_or_else(|| GraphEditError::NodeMissing(to_node.to_string()))?;

    let from_def = get_node_type(registry, &from.node_type)
        .ok_or_else(|| GraphEditError::UnknownType(from.node_type.clone()))?;
    let to_def = get_node_type(registry, &to.node_type)
        .ok_or_else(|| GraphEditError::UnknownType(to.node_type.clone()))?;

    let used_inputs: HashSet<&str> = graph
        .edges
        .values()
        .filter(|edge| edge.to.node == to_node)
        .map(|edge| edge.to.port.as_str())
        .collect();

    let outputs: Vec<_> = from_def.outputs.values().collect();
    let inputs: Vec<_> = to_def
        .inputs
        .values()
        .filter(|port| !used_inputs.contains(port.id.as_str()))
        .collect();

    // Prefer matching signal types.
    for out in &outputs {
        for inp in &inputs {
            if out.signal_type == inp.signal_type {
                return Ok((out.id.clone(), inp.id.clone()));
            }
        }
    }

    // Otherwise first free pair.
    if let (Some(out), Some(inp)) = (outputs.first(), inputs.first()) {
        return Ok((out.id.clone(), inp.id.clone()));
    }

    Err(GraphEditError::NoPortPair {
        from: from_node.to_string(),
        to: to_node.to_string(),
    })
}

/// Serializes an Imp graph as pretty Aura JSON with `type: "imp-graph"`.
pub fn graph_to_aura_json(graph: &Graph) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(graph)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.insert(
            "type".into(),
            serde_json::Value::String(String::from("imp-graph")),
        );
    }
    serde_json::to_string_pretty(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_imp::aura_registry;

    fn empty_graph() -> Graph {
        Graph {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }

    #[test]
    fn unique_node_id_increments() {
        let mut graph = empty_graph();
        add_node(&mut graph, "sine", "sine").unwrap();
        assert_eq!(unique_node_id(&graph, "sine"), "sine_2");
        add_node(&mut graph, "sine_2", "sine").unwrap();
        assert_eq!(unique_node_id(&graph, "sine"), "sine_3");
    }

    #[test]
    fn remove_node_drops_incident_edges() {
        let mut graph = empty_graph();
        add_node(&mut graph, "a", "parameter").unwrap();
        add_node(&mut graph, "b", "sine").unwrap();
        add_edge(&mut graph, "a", "value", "b", "frequency").unwrap();
        remove_node(&mut graph, "a").unwrap();
        assert!(!graph.nodes.contains_key("a"));
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn rename_rewrites_edges() {
        let mut graph = empty_graph();
        add_node(&mut graph, "a", "parameter").unwrap();
        add_node(&mut graph, "b", "sine").unwrap();
        add_edge(&mut graph, "a", "value", "b", "frequency").unwrap();
        rename_node(&mut graph, "a", "freq").unwrap();
        let edge = graph.edges.values().next().unwrap();
        assert_eq!(edge.from.node, "freq");
        assert!(graph.nodes.contains_key("freq"));
        assert!(!graph.nodes.contains_key("a"));
    }

    #[test]
    fn auto_pair_prefers_matching_types() {
        let registry = aura_registry().expect("aura registry");
        let mut graph = empty_graph();
        add_node(&mut graph, "freq", "parameter").unwrap();
        add_node(&mut graph, "osc", "sine").unwrap();
        let (from_port, to_port) =
            auto_pair_ports(&registry, &graph, "freq", "osc").expect("pair");
        assert_eq!(from_port, "value");
        assert_eq!(to_port, "frequency");
    }

    #[test]
    fn remove_edges_between_clears_pair() {
        let mut graph = empty_graph();
        add_node(&mut graph, "a", "parameter").unwrap();
        add_node(&mut graph, "b", "sine").unwrap();
        add_edge(&mut graph, "a", "value", "b", "frequency").unwrap();
        add_edge(&mut graph, "a", "value", "b", "time").unwrap();
        assert_eq!(remove_edges_between(&mut graph, "a", "b"), 2);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn graph_to_aura_json_sets_type() {
        let graph = empty_graph();
        let json = graph_to_aura_json(&graph).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "imp-graph");
        assert!(value.get("nodes").is_some());
    }
}
