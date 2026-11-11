//! Automatic layered layout for Imp graphs using petgraph.

use std::collections::{BTreeMap, BTreeSet};

use iced::{Point, Size};
use imp_core_types::{Graph, NodeId, NodeTypeId};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// Default node box size in layout space (simple connection mode).
pub const NODE_SIZE: Size = Size::new(148.0, 56.0);

/// Space reserved for node id + type above the port list in precise mode.
pub const NODE_HEADER_HEIGHT: f32 = 40.0;
/// Vertical pitch between consecutive port rows in precise mode.
pub const PORT_SPACING: f32 = 18.0;
/// Slightly wider than simple-mode boxes so input/output labels fit.
pub const PRECISE_NODE_WIDTH: f32 = 168.0;

const H_GAP: f32 = 72.0;
/// Minimum vertical gap between consecutive same-rank node boxes.
pub const V_GAP: f32 = 36.0;
const MARGIN: f32 = 24.0;

/// Computes the rendered node box size for simple or precise connection mode.
pub fn node_size_for(input_count: usize, output_count: usize, precise: bool) -> Size {
    if !precise {
        return NODE_SIZE;
    }
    let rows = input_count.max(output_count).max(1) as f32;
    let height = (NODE_HEADER_HEIGHT + rows * PORT_SPACING + 4.0).max(NODE_SIZE.height);
    Size::new(PRECISE_NODE_WIDTH, height)
}

/// A node placed by automatic layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutNode {
    pub id: NodeId,
    pub node_type: NodeTypeId,
    /// Top-left corner of the node box in layout space.
    pub position: Point,
    /// Rendered box size used for packing and content bounds.
    pub size: Size,
}

/// Node-level DAG layout produced from an Imp [`Graph`].
#[derive(Debug, Clone, PartialEq)]
pub struct GraphLayout {
    pub nodes: Vec<LaidOutNode>,
    /// Unique directed edges as `(from_node, to_node)` (ports ignored).
    pub edges: Vec<(NodeId, NodeId)>,
    /// Default simple-mode box size (callers should prefer per-node `LaidOutNode.size`).
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
///
/// `node_sizes` supplies rendered box sizes keyed by node id. Missing entries
/// fall back to [`NODE_SIZE`]. Same-rank nodes are packed top-to-bottom using
/// each node's height plus [`V_GAP`].
pub fn layout_graph(graph: &Graph, node_sizes: &BTreeMap<NodeId, Size>) -> GraphLayout {
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

    let column_width = graph
        .nodes
        .keys()
        .map(|id| node_sizes.get(id).copied().unwrap_or(NODE_SIZE).width)
        .fold(NODE_SIZE.width, f32::max);

    let mut laid_out = Vec::with_capacity(graph.nodes.len());
    let mut max_x = MARGIN;
    let mut max_y = MARGIN;

    for (rank, ids) in &by_rank {
        let x = MARGIN + (*rank as f32) * (column_width + H_GAP);
        let mut y = MARGIN;
        for id in ids {
            let size = node_sizes.get(id).copied().unwrap_or(NODE_SIZE);
            let node_type = graph
                .nodes
                .get(id)
                .map(|node| node.node_type.clone())
                .unwrap_or_default();
            laid_out.push(LaidOutNode {
                id: id.clone(),
                node_type,
                position: Point::new(x, y),
                size,
            });
            max_x = max_x.max(x + size.width);
            max_y = max_y.max(y + size.height);
            y += size.height + V_GAP;
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

    fn empty_sizes() -> BTreeMap<NodeId, Size> {
        BTreeMap::new()
    }

    #[test]
    fn sine_layout_places_all_nodes_with_signal_flow_left_to_right() {
        let layout = layout_graph(&sine_graph(), &empty_sizes());

        assert_eq!(layout.nodes.len(), 4);
        let freq = layout.position_of("freq").expect("freq");
        let time = layout.position_of("time").expect("time");
        let osc = layout.position_of("osc").expect("osc");
        let out = layout.position_of("out").expect("out");

        assert!(freq.x < osc.x, "freq should be left of osc");
        assert!(time.x < osc.x, "time should be left of osc");
        assert!(osc.x < out.x, "osc should be left of out");

        for (i, a) in layout.nodes.iter().enumerate() {
            for b in layout.nodes.iter().skip(i + 1) {
                let ax = a.position.x + a.size.width / 2.0;
                let ay = a.position.y + a.size.height / 2.0;
                let bx = b.position.x + b.size.width / 2.0;
                let by = b.position.y + b.size.height / 2.0;
                let dx = ax - bx;
                let dy = ay - by;
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

        let layout = layout_graph(&graph, &empty_sizes());
        let freq_to_osc = layout
            .edges
            .iter()
            .filter(|(from, to)| from == "freq" && to == "osc")
            .count();
        assert_eq!(freq_to_osc, 1);
    }

    #[test]
    fn precise_sizes_pack_same_rank_without_overlap() {
        // Two source nodes share rank 0 in the sine fixture (freq, time).
        let graph = sine_graph();
        let short = node_size_for(0, 1, true);
        let tall = node_size_for(4, 1, true);
        assert!(tall.height > short.height);
        assert!(tall.height > NODE_SIZE.height + V_GAP);

        let mut sizes = BTreeMap::new();
        sizes.insert("freq".into(), tall);
        sizes.insert("time".into(), short);
        sizes.insert("osc".into(), node_size_for(2, 1, true));
        sizes.insert("out".into(), node_size_for(1, 0, true));

        let layout = layout_graph(&graph, &sizes);
        let freq = layout
            .nodes
            .iter()
            .find(|n| n.id == "freq")
            .expect("freq");
        let time = layout
            .nodes
            .iter()
            .find(|n| n.id == "time")
            .expect("time");

        assert_eq!(freq.size, tall);
        assert_eq!(time.size, short);
        assert!((freq.position.x - time.position.x).abs() < 1e-3);

        // Sorted by id: freq then time.
        assert!(freq.position.y < time.position.y);
        let gap = time.position.y - (freq.position.y + freq.size.height);
        assert!(
            (gap - V_GAP).abs() < 1e-3,
            "expected V_GAP between same-rank boxes, got {gap}"
        );

        for (i, a) in layout.nodes.iter().enumerate() {
            for b in layout.nodes.iter().skip(i + 1) {
                let overlap_x = a.position.x < b.position.x + b.size.width
                    && b.position.x < a.position.x + a.size.width;
                let overlap_y = a.position.y < b.position.y + b.size.height
                    && b.position.y < a.position.y + a.size.height;
                assert!(
                    !(overlap_x && overlap_y),
                    "nodes {} and {} overlap",
                    a.id,
                    b.id
                );
            }
        }

        let bottom = layout
            .nodes
            .iter()
            .map(|n| n.position.y + n.size.height)
            .fold(0.0, f32::max);
        let right = layout
            .nodes
            .iter()
            .map(|n| n.position.x + n.size.width)
            .fold(0.0, f32::max);
        assert!(
            (layout.content_size.height - (bottom + layout.origin.y)).abs() < 1e-3,
            "content_size.height {} should equal bottom {} + origin.y",
            layout.content_size.height,
            bottom
        );
        assert!(
            (layout.content_size.width - (right + layout.origin.x)).abs() < 1e-3,
            "content_size.width {} should equal right {} + origin.x",
            layout.content_size.width,
            right
        );
        assert!(right >= PRECISE_NODE_WIDTH + layout.origin.x - 1e-3);
    }
}
