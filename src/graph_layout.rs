//! Automatic layered layout for Imp graphs using petgraph.

use std::collections::{BTreeMap, BTreeSet};

use iced::{Point, Size};
use imp_core_types::{Graph, NodeId, NodeTypeId};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// Default node box size in layout space.
pub const NODE_SIZE: Size = Size::new(148.0, 56.0);

const H_GAP: f32 = 72.0;
const V_GAP: f32 = 36.0;
const MARGIN: f32 = 24.0;

/// A node placed by automatic layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutNode {
    pub id: NodeId,
    pub node_type: NodeTypeId,
    /// Top-left corner of the node box in layout space.
    pub position: Point,
}

/// Node-level DAG layout produced from an Imp [`Graph`].
#[derive(Debug, Clone, PartialEq)]
pub struct GraphLayout {
    pub nodes: Vec<LaidOutNode>,
    /// Unique directed edges as `(from_node, to_node)` (ports ignored).
    pub edges: Vec<(NodeId, NodeId)>,
    pub node_size: Size,
    /// Inclusive origin of content (typically the margin origin).
    pub origin: Point,
    pub content_size: Size,
}

impl GraphLayout {
    /// Center of the laid-out content in layout space.
    pub fn content_center(&self) -> Point {
        Point::new(
            self.origin.x + self.content_size.width / 2.0,
            self.origin.y + self.content_size.height / 2.0,
        )
    }

    /// Looks up a node's top-left position by id.
    pub fn position_of(&self, id: &str) -> Option<Point> {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.position)
    }
}

/// Builds a left-to-right layered layout for `graph`.
///
/// Ranks use longest-path distance from sources on a petgraph [`DiGraph`].
/// Cycles are tolerated: nodes not ordered by toposort receive a fallback rank.
pub fn layout_graph(graph: &Graph) -> GraphLayout {
    let mut digraph: DiGraph<NodeId, ()> = DiGraph::new();
    let mut index_by_id: BTreeMap<NodeId, NodeIndex> = BTreeMap::new();

    for id in graph.nodes.keys() {
        let index = digraph.add_node(id.clone());
        index_by_id.insert(id.clone(), index);
    }

    let mut edge_pairs: BTreeSet<(NodeId, NodeId)> = BTreeSet::new();
    for edge in graph.edges.values() {
        if edge.from.node == edge.to.node {
            continue;
        }
        if !index_by_id.contains_key(&edge.from.node)
            || !index_by_id.contains_key(&edge.to.node)
        {
            continue;
        }
        edge_pairs.insert((edge.from.node.clone(), edge.to.node.clone()));
    }

    for (from, to) in &edge_pairs {
        let from_index = index_by_id[from];
        let to_index = index_by_id[to];
        digraph.add_edge(from_index, to_index, ());
    }

    let ranks = assign_ranks(&digraph);

    let mut by_rank: BTreeMap<u32, Vec<NodeId>> = BTreeMap::new();
    for (index, rank) in &ranks {
        by_rank
            .entry(*rank)
            .or_default()
            .push(digraph[*index].clone());
    }
    for nodes in by_rank.values_mut() {
        nodes.sort();
    }

    let mut laid_out = Vec::with_capacity(graph.nodes.len());
    let mut max_x = MARGIN;
    let mut max_y = MARGIN;

    for (rank, ids) in &by_rank {
        let x = MARGIN + (*rank as f32) * (NODE_SIZE.width + H_GAP);
        for (row, id) in ids.iter().enumerate() {
            let y = MARGIN + (row as f32) * (NODE_SIZE.height + V_GAP);
            let node_type = graph
                .nodes
                .get(id)
                .map(|node| node.node_type.clone())
                .unwrap_or_default();
            laid_out.push(LaidOutNode {
                id: id.clone(),
                node_type,
                position: Point::new(x, y),
            });
            max_x = max_x.max(x + NODE_SIZE.width);
            max_y = max_y.max(y + NODE_SIZE.height);
        }
    }

    // Stable draw order by id for deterministic rendering.
    laid_out.sort_by(|a, b| a.id.cmp(&b.id));

    let edges: Vec<(NodeId, NodeId)> = edge_pairs.into_iter().collect();

    GraphLayout {
        nodes: laid_out,
        edges,
        node_size: NODE_SIZE,
        origin: Point::new(MARGIN, MARGIN),
        content_size: Size::new(
            (max_x + MARGIN).max(MARGIN * 2.0),
            (max_y + MARGIN).max(MARGIN * 2.0),
        ),
    }
}

fn assign_ranks(graph: &DiGraph<NodeId, ()>) -> BTreeMap<NodeIndex, u32> {
    let mut ranks: BTreeMap<NodeIndex, u32> = BTreeMap::new();

    match toposort(graph, None) {
        Ok(order) => {
            for node in order {
                let rank = graph
                    .edges_directed(node, Direction::Incoming)
                    .map(|edge| ranks.get(&edge.source()).copied().unwrap_or(0) + 1)
                    .max()
                    .unwrap_or(0);
                ranks.insert(node, rank);
            }
        }
        Err(_) => {
            // Best-effort: Kahn-style partial ranking, then park leftovers.
            let mut indegree: BTreeMap<NodeIndex, usize> = graph
                .node_indices()
                .map(|n| {
                    (
                        n,
                        graph.edges_directed(n, Direction::Incoming).count(),
                    )
                })
                .collect();
            let mut ready: Vec<NodeIndex> = indegree
                .iter()
                .filter_map(|(n, d)| (*d == 0).then_some(*n))
                .collect();
            ready.sort_by_key(|n| graph[*n].clone());

            while let Some(node) = ready.pop() {
                let rank = graph
                    .edges_directed(node, Direction::Incoming)
                    .filter_map(|edge| ranks.get(&edge.source()).copied())
                    .map(|r| r + 1)
                    .max()
                    .unwrap_or(0);
                ranks.insert(node, rank);

                let mut unlocked = Vec::new();
                for edge in graph.edges_directed(node, Direction::Outgoing) {
                    let target = edge.target();
                    if let Some(degree) = indegree.get_mut(&target) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 && !ranks.contains_key(&target) {
                            unlocked.push(target);
                        }
                    }
                }
                unlocked.sort_by_key(|n| graph[*n].clone());
                ready.extend(unlocked);
            }

            let fallback = ranks.values().copied().max().unwrap_or(0).saturating_add(1);
            for node in graph.node_indices() {
                ranks.entry(node).or_insert(fallback);
            }
        }
    }

    ranks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sine_graph() -> Graph {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sine.json");
        let text = std::fs::read_to_string(path).expect("read sine fixture");
        serde_json::from_str(&text).expect("parse sine fixture")
    }

    #[test]
    fn sine_layout_places_all_nodes_with_signal_flow_left_to_right() {
        let layout = layout_graph(&sine_graph());

        assert_eq!(layout.nodes.len(), 4);
        let freq = layout.position_of("freq").expect("freq");
        let time = layout.position_of("time").expect("time");
        let osc = layout.position_of("osc").expect("osc");
        let out = layout.position_of("out").expect("out");

        assert!(freq.x < osc.x, "freq should be left of osc");
        assert!(time.x < osc.x, "time should be left of osc");
        assert!(osc.x < out.x, "osc should be left of out");

        let centers: Vec<(f32, f32)> = layout
            .nodes
            .iter()
            .map(|node| {
                (
                    node.position.x + layout.node_size.width / 2.0,
                    node.position.y + layout.node_size.height / 2.0,
                )
            })
            .collect();
        for (i, a) in centers.iter().enumerate() {
            for b in centers.iter().skip(i + 1) {
                let dx = a.0 - b.0;
                let dy = a.1 - b.1;
                assert!(
                    dx * dx + dy * dy > 1.0,
                    "node centers should not overlap"
                );
            }
        }
    }

    #[test]
    fn collapses_parallel_port_edges_to_one_node_edge() {
        let mut graph = sine_graph();
        graph.edges.insert(
            "freq_to_osc_again".into(),
            imp_core_types::Edge {
                from: imp_core_types::PortReference {
                    node: "freq".into(),
                    port: "value".into(),
                },
                to: imp_core_types::PortReference {
                    node: "osc".into(),
                    port: "phase".into(),
                },
            },
        );

        let layout = layout_graph(&graph);
        let freq_to_osc = layout
            .edges
            .iter()
            .filter(|(from, to)| from == "freq" && to == "osc")
            .count();
        assert_eq!(freq_to_osc, 1);
    }
}
