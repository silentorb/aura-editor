//! Iced Canvas DAG view with pan/zoom over an automatic layout.

use iced::mouse;
use iced::widget::canvas::{self, path::Builder, Frame, Geometry, Path, Stroke, Text};
use iced::widget::Canvas;
use iced::{Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::document::DocumentId;
use crate::graph_layout::{layout_graph, GraphLayout};
use imp_core_types::Graph;

const MIN_SCALE: f32 = 0.25;
const MAX_SCALE: f32 = 4.0;
const ZOOM_STEP: f32 = 1.1;
const NODE_RADIUS: f32 = 8.0;

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

/// Canvas program that draws a laid-out Imp graph.
#[derive(Debug, Clone)]
pub struct GraphView {
    layout: GraphLayout,
    document_key: u64,
}

impl GraphView {
    /// Creates a view for `graph` belonging to `document_id`.
    pub fn new(document_id: DocumentId, graph: &Graph) -> Self {
        Self {
            layout: layout_graph(graph),
            document_key: document_id.get(),
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
    }
}

/// Pan/zoom camera stored in the canvas widget tree.
#[derive(Debug, Clone)]
pub struct ViewState {
    translation: Vector,
    scale: f32,
    drag_last: Option<Point>,
    document_key: Option<u64>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            translation: Vector::ZERO,
            scale: 1.0,
            drag_last: None,
            document_key: None,
        }
    }
}

/// Builds a fill-sized canvas for the active graph document.
pub fn graph_canvas<'a, Message: 'a>(
    document_id: DocumentId,
    graph: &'a Graph,
) -> Element<'a, Message> {
    Canvas::new(GraphView::new(document_id, graph))
        .width(Fill)
        .height(Fill)
        .into()
}

impl<Message> canvas::Program<Message> for GraphView {
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
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(position) = cursor_in_bounds {
                    state.drag_last = Some(position);
                    return Some(canvas::Action::capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if state.drag_last.is_some() {
                    state.drag_last = None;
                    return Some(canvas::Action::capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some(last), Some(position)) = (state.drag_last, cursor_in_bounds) {
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
        let node_fill = palette.background.weak.color;
        let node_border = palette.background.strong.color;
        let text_color = palette.background.base.text;
        let muted_text = palette.background.weak.text;

        let stroke = Stroke::default()
            .with_width(1.5 / scale.max(0.25))
            .with_color(edge_color);

        let mut edges = Builder::new();
        for (from_id, to_id) in &self.layout.edges {
            let Some(from) = self.layout.position_of(from_id) else {
                continue;
            };
            let Some(to) = self.layout.position_of(to_id) else {
                continue;
            };
            let start = Point::new(
                from.x + self.layout.node_size.width,
                from.y + self.layout.node_size.height / 2.0,
            );
            let end = Point::new(to.x, to.y + self.layout.node_size.height / 2.0);
            edges.move_to(start);
            edges.line_to(end);
        }
        frame.stroke(&edges.build(), stroke);

        let border = Stroke::default()
            .with_width(1.0 / scale.max(0.25))
            .with_color(node_border);

        for node in &self.layout.nodes {
            let rect = Path::rounded_rectangle(
                node.position,
                self.layout.node_size,
                NODE_RADIUS.into(),
            );
            frame.fill(&rect, node_fill);
            frame.stroke(&rect, border);

            let id_pos = Point::new(node.position.x + 10.0, node.position.y + 10.0);
            frame.fill_text(Text {
                content: node.id.clone(),
                position: id_pos,
                color: text_color,
                size: Pixels(14.0),
                font: Font::DEFAULT,
                ..Text::default()
            });

            let type_pos = Point::new(node.position.x + 10.0, node.position.y + 30.0);
            frame.fill_text(Text {
                content: node.node_type.clone(),
                position: type_pos,
                color: muted_text,
                size: Pixels(12.0),
                font: Font::DEFAULT,
                ..Text::default()
            });
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag_last.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_layout::layout_graph;
    use crate::load_graph_from_path;
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
        let graph = load_graph_from_path(path).unwrap();
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
