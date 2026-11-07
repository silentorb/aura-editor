//! Iced Canvas DAG view with pan/zoom and modal graph editing.

use std::collections::BTreeMap;

use iced::mouse;
use iced::widget::canvas::{self, path::Builder, Frame, Geometry, Path, Stroke, Text};
use iced::widget::Canvas;
use iced::{Color, Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::document::DocumentId;
use crate::graph_layout::{layout_graph, GraphLayout, NODE_SIZE};
use crate::graph_ui::{ConnectionMode, GraphTool, GraphUiState};
use imp_core_types::{EdgeId, Graph, NodeId};

const MIN_SCALE: f32 = 0.25;
const MAX_SCALE: f32 = 4.0;
const ZOOM_STEP: f32 = 1.1;
const NODE_RADIUS: f32 = 8.0;
const PORT_RADIUS: f32 = 5.0;
const PORT_HIT_RADIUS: f32 = 10.0;
const PORT_SPACING: f32 = 18.0;
const EDGE_HIT_DISTANCE: f32 = 8.0;

/// Input/output port ids for a node type instance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodePortInfo {
    /// Input port ids (top→bottom).
    pub inputs: Vec<String>,
    /// Output port ids (top→bottom).
    pub outputs: Vec<String>,
}

/// Port catalog keyed by node id.
pub type PortCatalog = BTreeMap<NodeId, NodePortInfo>;

/// Canvas edit events emitted by the graph view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphEdit {
    /// Select a node (or clear when `None`).
    SelectNode {
        /// Document being edited.
        document_id: DocumentId,
        /// Node id, or `None` to clear.
        node_id: Option<String>,
    },
    /// Select an edge (or clear when `None`).
    SelectEdge {
        /// Document being edited.
        document_id: DocumentId,
        /// Edge id, or `None` to clear.
        edge_id: Option<String>,
    },
    /// Insert a node of the current Add type.
    AddNode {
        /// Document being edited.
        document_id: DocumentId,
    },
    /// Simple-mode connect between two nodes.
    ConnectNodes {
        /// Document being edited.
        document_id: DocumentId,
        /// Source node.
        from: String,
        /// Target node.
        to: String,
    },
    /// Precise-mode connect between ports.
    ConnectPorts {
        /// Document being edited.
        document_id: DocumentId,
        /// Source node.
        from_node: String,
        /// Source output port.
        from_port: String,
        /// Target node.
        to_node: String,
        /// Target input port.
        to_port: String,
    },
    /// Begin a connect gesture.
    BeginConnect {
        /// Document being edited.
        document_id: DocumentId,
        /// Source node.
        from_node: String,
        /// Source port when precise.
        from_port: Option<String>,
    },
    /// Cancel an in-progress connect.
    CancelConnect {
        /// Document being edited.
        document_id: DocumentId,
    },
    /// Erase a node and incident edges.
    EraseNode {
        /// Document being edited.
        document_id: DocumentId,
        /// Node to remove.
        node_id: String,
    },
    /// Erase a single edge by id.
    EraseEdge {
        /// Document being edited.
        document_id: DocumentId,
        /// Edge to remove.
        edge_id: String,
    },
    /// Erase all edges between two nodes (simple-mode line).
    EraseEdgesBetween {
        /// Document being edited.
        document_id: DocumentId,
        /// One endpoint.
        a: String,
        /// Other endpoint.
        b: String,
    },
}

/// Clamps canvas zoom to the supported range.
pub fn clamp_scale(scale: f32) -> f32 {
    scale.clamp(MIN_SCALE, MAX_SCALE)
}

/// Pans by a screen-space delta.
pub fn pan_by(translation: Vector, delta: Vector) -> Vector {
    translation + delta
}

/// Zooms so the layout point under `cursor` stays fixed.
pub fn zoom_at_point(
    translation: Vector,
    scale: f32,
    cursor: Point,
    factor: f32,
) -> (Vector, f32) {
    let new_scale = clamp_scale(scale * factor);
    if (new_scale - scale).abs() < f32::EPSILON {
        return (translation, scale);
    }

    let world = Point::new(
        (cursor.x - translation.x) / scale,
        (cursor.y - translation.y) / scale,
    );
    let new_translation = Vector::new(
        cursor.x - world.x * new_scale,
        cursor.y - world.y * new_scale,
    );
    (new_translation, new_scale)
}

/// Translation that centers `layout` in `viewport` at `scale`.
pub fn center_translation(layout: &GraphLayout, viewport: Size, scale: f32) -> Vector {
    let center = layout.content_center();
    Vector::new(
        viewport.width / 2.0 - center.x * scale,
        viewport.height / 2.0 - center.y * scale,
    )
}

fn node_size_for(ports: Option<&NodePortInfo>, precise: bool) -> Size {
    if !precise {
        return NODE_SIZE;
    }
    let Some(ports) = ports else {
        return NODE_SIZE;
    };
    let rows = ports.inputs.len().max(ports.outputs.len()).max(1) as f32;
    let height = (24.0 + rows * PORT_SPACING).max(NODE_SIZE.height);
    Size::new(NODE_SIZE.width, height)
}

fn port_position(
    node_pos: Point,
    size: Size,
    index: usize,
    count: usize,
    is_output: bool,
) -> Point {
    let span = (count.max(1) as f32) * PORT_SPACING;
    let start_y = node_pos.y + (size.height - span) / 2.0 + PORT_SPACING / 2.0;
    let y = start_y + index as f32 * PORT_SPACING;
    let x = if is_output {
        node_pos.x + size.width
    } else {
        node_pos.x
    };
    Point::new(x, y)
}

/// Canvas program that draws and edits a laid-out Imp graph.
#[derive(Debug, Clone)]
pub struct GraphView {
    layout: GraphLayout,
    document_key: u64,
    document_id: DocumentId,
    tool: GraphTool,
    connection_mode: ConnectionMode,
    selected_node: Option<String>,
    selected_edge: Option<String>,
    connect_from_node: Option<String>,
    connect_from_port: Option<String>,
    ports: PortCatalog,
    /// Full edges for precise drawing / hit-testing: (edge_id, from_node, from_port, to_node, to_port).
    edges: Vec<(EdgeId, NodeId, String, NodeId, String)>,
}

impl GraphView {
    /// Creates a view for `graph` belonging to `document_id`.
    pub fn new(
        document_id: DocumentId,
        graph: &Graph,
        ui: &GraphUiState,
        ports: PortCatalog,
    ) -> Self {
        let edges = graph
            .edges
            .iter()
            .map(|(id, edge)| {
                (
                    id.clone(),
                    edge.from.node.clone(),
                    edge.from.port.clone(),
                    edge.to.node.clone(),
                    edge.to.port.clone(),
                )
            })
            .collect();
        Self {
            layout: layout_graph(graph),
            document_key: document_id.get(),
            document_id,
            tool: ui.tool,
            connection_mode: ui.connection_mode,
            selected_node: ui.selected_node.clone(),
            selected_edge: ui.selected_edge.clone(),
            connect_from_node: ui.connect_draft.as_ref().map(|d| d.from_node.clone()),
            connect_from_port: ui
                .connect_draft
                .as_ref()
                .and_then(|d| d.from_port.clone()),
            ports,
            edges,
        }
    }

    fn camera_for(&self, state: &ViewState, bounds: Size) -> (Vector, f32) {
        if state.document_key == Some(self.document_key) {
            (state.translation, state.scale)
        } else {
            (center_translation(&self.layout, bounds, 1.0), 1.0)
        }
    }

    fn sync_document(&self, state: &mut ViewState, bounds: Size) {
        if state.document_key == Some(self.document_key) {
            return;
        }
        state.document_key = Some(self.document_key);
        state.scale = 1.0;
        state.translation = center_translation(&self.layout, bounds, 1.0);
        state.drag_last = None;
        state.panning = false;
    }

    fn screen_to_world(&self, state: &ViewState, bounds: Size, screen: Point) -> Point {
        let (translation, scale) = self.camera_for(state, bounds);
        Point::new(
            (screen.x - translation.x) / scale,
            (screen.y - translation.y) / scale,
        )
    }

    fn size_of(&self, node_id: &str) -> Size {
        node_size_for(
            self.ports.get(node_id),
            self.connection_mode == ConnectionMode::Precise,
        )
    }

    fn hit_node(&self, world: Point) -> Option<NodeId> {
        for node in self.layout.nodes.iter().rev() {
            let size = self.size_of(&node.id);
            let rect = Rectangle::new(node.position, size);
            if rect.contains(world) {
                return Some(node.id.clone());
            }
        }
        None
    }

    fn hit_port(&self, world: Point) -> Option<(NodeId, String, bool)> {
        if self.connection_mode != ConnectionMode::Precise {
            return None;
        }
        for node in &self.layout.nodes {
            let Some(info) = self.ports.get(&node.id) else {
                continue;
            };
            let size = self.size_of(&node.id);
            for (index, port) in info.inputs.iter().enumerate() {
                let pos = port_position(node.position, size, index, info.inputs.len(), false);
                if (pos.x - world.x).hypot(pos.y - world.y) <= PORT_HIT_RADIUS {
                    return Some((node.id.clone(), port.clone(), false));
                }
            }
            for (index, port) in info.outputs.iter().enumerate() {
                let pos = port_position(node.position, size, index, info.outputs.len(), true);
                if (pos.x - world.x).hypot(pos.y - world.y) <= PORT_HIT_RADIUS {
                    return Some((node.id.clone(), port.clone(), true));
                }
            }
        }
        None
    }

    fn edge_endpoints(&self, from: &str, from_port: &str, to: &str, to_port: &str) -> Option<(Point, Point)> {
        let from_node = self.layout.nodes.iter().find(|n| n.id == from)?;
        let to_node = self.layout.nodes.iter().find(|n| n.id == to)?;
        let from_size = self.size_of(from);
        let to_size = self.size_of(to);

        let start = if self.connection_mode == ConnectionMode::Precise {
            let info = self.ports.get(from)?;
            let index = info.outputs.iter().position(|p| p == from_port).unwrap_or(0);
            port_position(from_node.position, from_size, index, info.outputs.len().max(1), true)
        } else {
            Point::new(
                from_node.position.x + from_size.width,
                from_node.position.y + from_size.height / 2.0,
            )
        };

        let end = if self.connection_mode == ConnectionMode::Precise {
            let info = self.ports.get(to)?;
            let index = info.inputs.iter().position(|p| p == to_port).unwrap_or(0);
            port_position(to_node.position, to_size, index, info.inputs.len().max(1), false)
        } else {
            Point::new(
                to_node.position.x,
                to_node.position.y + to_size.height / 2.0,
            )
        };

        Some((start, end))
    }

    fn hit_edge(&self, world: Point) -> Option<(EdgeId, NodeId, NodeId)> {
        let mut best: Option<(f32, EdgeId, NodeId, NodeId)> = None;
        if self.connection_mode == ConnectionMode::Simple {
            for (from, to) in &self.layout.edges {
                let Some((start, end)) = self.edge_endpoints(from, "", to, "") else {
                    continue;
                };
                let dist = point_segment_distance(world, start, end);
                if dist <= EDGE_HIT_DISTANCE
                    && best.as_ref().is_none_or(|(best_d, ..)| dist < *best_d)
                {
                    // Use first matching real edge id for the pair.
                    let edge_id = self
                        .edges
                        .iter()
                        .find(|(_, f, _, t, _)| f == from && t == to)
                        .map(|(id, ..)| id.clone())
                        .unwrap_or_else(|| format!("{from}->{to}"));
                    best = Some((dist, edge_id, from.clone(), to.clone()));
                }
            }
        } else {
            for (edge_id, from, from_port, to, to_port) in &self.edges {
                let Some((start, end)) =
                    self.edge_endpoints(from, from_port, to, to_port)
                else {
                    continue;
                };
                let dist = point_segment_distance(world, start, end);
                if dist <= EDGE_HIT_DISTANCE
                    && best.as_ref().is_none_or(|(best_d, ..)| dist < *best_d)
                {
                    best = Some((dist, edge_id.clone(), from.clone(), to.clone()));
                }
            }
        }
        best.map(|(_, id, a, b)| (id, a, b))
    }

    fn handle_click(&self, world: Point) -> Option<GraphEdit> {
        let document_id = self.document_id;
        match self.tool {
            GraphTool::Select => {
                if let Some(node) = self.hit_node(world) {
                    return Some(GraphEdit::SelectNode {
                        document_id,
                        node_id: Some(node),
                    });
                }
                if let Some((edge_id, _, _)) = self.hit_edge(world) {
                    return Some(GraphEdit::SelectEdge {
                        document_id,
                        edge_id: Some(edge_id),
                    });
                }
                Some(GraphEdit::SelectNode {
                    document_id,
                    node_id: None,
                })
            }
            GraphTool::Add => {
                if self.hit_node(world).is_some() || self.hit_edge(world).is_some() {
                    return None;
                }
                Some(GraphEdit::AddNode { document_id })
            }
            GraphTool::Connect => {
                if self.connection_mode == ConnectionMode::Precise {
                    if let Some((node, port, is_output)) = self.hit_port(world) {
                        if is_output {
                            return Some(GraphEdit::BeginConnect {
                                document_id,
                                from_node: node,
                                from_port: Some(port),
                            });
                        }
                        if let (Some(from_node), Some(from_port)) =
                            (&self.connect_from_node, &self.connect_from_port)
                        {
                            return Some(GraphEdit::ConnectPorts {
                                document_id,
                                from_node: from_node.clone(),
                                from_port: from_port.clone(),
                                to_node: node,
                                to_port: port,
                            });
                        }
                    }
                    return Some(GraphEdit::CancelConnect { document_id });
                }

                // Simple: click source then target.
                let Some(node) = self.hit_node(world) else {
                    return Some(GraphEdit::CancelConnect { document_id });
                };
                if let Some(from) = &self.connect_from_node {
                    if from == &node {
                        return Some(GraphEdit::CancelConnect { document_id });
                    }
                    return Some(GraphEdit::ConnectNodes {
                        document_id,
                        from: from.clone(),
                        to: node,
                    });
                }
                Some(GraphEdit::BeginConnect {
                    document_id,
                    from_node: node,
                    from_port: None,
                })
            }
            GraphTool::Erase => {
                if let Some(node) = self.hit_node(world) {
                    return Some(GraphEdit::EraseNode {
                        document_id,
                        node_id: node,
                    });
                }
                if let Some((edge_id, a, b)) = self.hit_edge(world) {
                    if self.connection_mode == ConnectionMode::Simple {
                        return Some(GraphEdit::EraseEdgesBetween {
                            document_id,
                            a,
                            b,
                        });
                    }
                    return Some(GraphEdit::EraseEdge {
                        document_id,
                        edge_id,
                    });
                }
                None
            }
        }
    }
}

fn point_segment_distance(p: Point, a: Point, b: Point) -> f32 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len_sq = abx * abx + aby * aby;
    if len_sq < f32::EPSILON {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = ((p.x - a.x) * abx + (p.y - a.y) * aby) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj = Point::new(a.x + t * abx, a.y + t * aby);
    (p.x - proj.x).hypot(p.y - proj.y)
}

/// Pan/zoom camera stored in the canvas widget tree.
#[derive(Debug, Clone)]
pub struct ViewState {
    translation: Vector,
    scale: f32,
    drag_last: Option<Point>,
    panning: bool,
    document_key: Option<u64>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            translation: Vector::ZERO,
            scale: 1.0,
            drag_last: None,
            panning: false,
            document_key: None,
        }
    }
}

/// Builds a fill-sized canvas for the active graph document.
pub fn graph_canvas<'a, Message>(
    document_id: DocumentId,
    graph: &'a Graph,
    ui: &'a GraphUiState,
    ports: PortCatalog,
) -> Element<'a, Message>
where
    Message: 'a + Clone + From<GraphEdit>,
{
    Canvas::new(GraphView::new(document_id, graph, ui, ports))
        .width(Fill)
        .height(Fill)
        .into()
}

impl<Message> canvas::Program<Message> for GraphView
where
    Message: Clone + From<GraphEdit>,
{
    type State = ViewState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        self.sync_document(state, bounds.size());

        let cursor_in_bounds = cursor.position_in(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(position) = cursor_in_bounds {
                    state.drag_last = Some(position);
                    state.panning = true;
                    return Some(canvas::Action::capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor_in_bounds?;
                let world = self.screen_to_world(state, bounds.size(), position);
                let hit = self.hit_node(world).is_some()
                    || self.hit_edge(world).is_some()
                    || self.hit_port(world).is_some()
                    || self.tool == GraphTool::Add;

                if !hit {
                    state.drag_last = Some(position);
                    state.panning = true;
                    return Some(canvas::Action::capture());
                }

                if let Some(edit) = self.handle_click(world) {
                    return Some(canvas::Action::publish(Message::from(edit)).and_capture());
                }
                return Some(canvas::Action::capture());
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if state.drag_last.is_some() || state.panning {
                    state.drag_last = None;
                    state.panning = false;
                    return Some(canvas::Action::capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (true, Some(last), Some(position)) =
                    (state.panning, state.drag_last, cursor_in_bounds)
                {
                    let delta = Vector::new(position.x - last.x, position.y - last.y);
                    state.translation = pan_by(state.translation, delta);
                    state.drag_last = Some(position);
                    return Some(canvas::Action::request_redraw().and_capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let position = cursor_in_bounds?;
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                if y.abs() < f32::EPSILON {
                    return None;
                }
                let factor = if y > 0.0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
                let (translation, scale) =
                    zoom_at_point(state.translation, state.scale, position, factor);
                state.translation = translation;
                state.scale = scale;
                return Some(canvas::Action::request_redraw().and_capture());
            }
            _ => {}
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let (translation, scale) = self.camera_for(state, bounds.size());
        let mut frame = Frame::new(renderer, bounds.size());

        frame.translate(translation);
        frame.scale(scale);

        let palette = theme.extended_palette();
        let edge_color = palette.secondary.base.color;
        let selected_edge = palette.primary.strong.color;
        let node_fill = palette.background.weak.color;
        let node_selected = palette.primary.weak.color;
        let node_border = palette.background.strong.color;
        let text_color = palette.background.base.text;
        let muted_text = palette.background.weak.text;
        let port_fill = palette.primary.base.color;

        let stroke_width = 1.5 / scale.max(0.25);

        if self.connection_mode == ConnectionMode::Precise {
            for (edge_id, from, from_port, to, to_port) in &self.edges {
                let Some((start, end)) = self.edge_endpoints(from, from_port, to, to_port) else {
                    continue;
                };
                let mut path = Builder::new();
                path.move_to(start);
                path.line_to(end);
                let color = if self.selected_edge.as_deref() == Some(edge_id.as_str()) {
                    selected_edge
                } else {
                    edge_color
                };
                frame.stroke(
                    &path.build(),
                    Stroke::default().with_width(stroke_width).with_color(color),
                );
            }
        } else {
            for (from_id, to_id) in &self.layout.edges {
                let Some((start, end)) = self.edge_endpoints(from_id, "", to_id, "") else {
                    continue;
                };
                let mut path = Builder::new();
                path.move_to(start);
                path.line_to(end);
                let selected = self.selected_edge.as_ref().is_some_and(|edge_id| {
                    self.edges.iter().any(|(id, f, _, t, _)| {
                        id == edge_id && f == from_id && t == to_id
                    })
                });
                let color = if selected { selected_edge } else { edge_color };
                frame.stroke(
                    &path.build(),
                    Stroke::default().with_width(stroke_width).with_color(color),
                );
            }
        }

        for node in &self.layout.nodes {
            let size = self.size_of(&node.id);
            let selected = self.selected_node.as_deref() == Some(node.id.as_str())
                || self.connect_from_node.as_deref() == Some(node.id.as_str());
            let fill = if selected { node_selected } else { node_fill };
            let rect = Path::rounded_rectangle(node.position, size, NODE_RADIUS.into());
            frame.fill(&rect, fill);
            frame.stroke(
                &rect,
                Stroke::default()
                    .with_width(1.0 / scale.max(0.25))
                    .with_color(node_border),
            );

            frame.fill_text(Text {
                content: node.id.clone(),
                position: Point::new(node.position.x + 10.0, node.position.y + 8.0),
                color: text_color,
                size: Pixels(14.0),
                font: Font::DEFAULT,
                ..Text::default()
            });
            frame.fill_text(Text {
                content: node.node_type.clone(),
                position: Point::new(node.position.x + 10.0, node.position.y + 26.0),
                color: muted_text,
                size: Pixels(12.0),
                font: Font::DEFAULT,
                ..Text::default()
            });

            if self.connection_mode == ConnectionMode::Precise {
                if let Some(info) = self.ports.get(&node.id) {
                    for (index, _) in info.inputs.iter().enumerate() {
                        let pos =
                            port_position(node.position, size, index, info.inputs.len(), false);
                        frame.fill(
                            &Path::circle(pos, PORT_RADIUS),
                            port_fill,
                        );
                    }
                    for (index, _) in info.outputs.iter().enumerate() {
                        let pos =
                            port_position(node.position, size, index, info.outputs.len(), true);
                        frame.fill(
                            &Path::circle(pos, PORT_RADIUS),
                            Color {
                                a: 0.9,
                                ..port_fill
                            },
                        );
                    }
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.panning {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            match self.tool {
                GraphTool::Add | GraphTool::Connect | GraphTool::Erase => {
                    mouse::Interaction::Crosshair
                }
                GraphTool::Select => mouse::Interaction::Pointer,
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_layout::layout_graph;
    use imp_core_types::Graph;
    use std::path::PathBuf;

    #[test]
    fn clamp_scale_limits_range() {
        assert_eq!(clamp_scale(0.01), MIN_SCALE);
        assert_eq!(clamp_scale(10.0), MAX_SCALE);
        assert_eq!(clamp_scale(1.5), 1.5);
    }

    #[test]
    fn pan_by_adds_delta() {
        let next = pan_by(Vector::new(10.0, 20.0), Vector::new(-3.0, 4.0));
        assert_eq!(next, Vector::new(7.0, 24.0));
    }

    #[test]
    fn zoom_at_point_keeps_world_point_stable() {
        let translation = Vector::new(50.0, 40.0);
        let scale = 1.0;
        let cursor = Point::new(100.0, 80.0);
        let (next_t, next_s) = zoom_at_point(translation, scale, cursor, 2.0);
        assert_eq!(next_s, 2.0);

        let world_before = Point::new(
            (cursor.x - translation.x) / scale,
            (cursor.y - translation.y) / scale,
        );
        let world_after = Point::new(
            (cursor.x - next_t.x) / next_s,
            (cursor.y - next_t.y) / next_s,
        );
        assert!((world_before.x - world_after.x).abs() < 1e-4);
        assert!((world_before.y - world_after.y).abs() < 1e-4);
    }

    #[test]
    fn center_translation_puts_content_center_at_viewport_center() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sine.json");
        let text = std::fs::read_to_string(path).expect("read sine fixture");
        let graph: Graph = serde_json::from_str(&text).expect("parse sine fixture");
        let layout = layout_graph(&graph);
        let viewport = Size::new(800.0, 600.0);
        let translation = center_translation(&layout, viewport, 1.0);
        let center = layout.content_center();
        let screen = Point::new(
            center.x * 1.0 + translation.x,
            center.y * 1.0 + translation.y,
        );
        assert!((screen.x - 400.0).abs() < 1e-3);
        assert!((screen.y - 300.0).abs() < 1e-3);
    }
}
