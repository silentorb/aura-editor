//! Piano-roll view: piano lane + time × pitch canvas for notation documents.

use aura_ext_notation::{
    is_black_key, semitone_label, snap_beats, Notation, NotationNote, NotationTool, SnapResolution,
};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{row, Canvas};
use iced::{Color, Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::document::DocumentId;
use crate::piano_roll_ui::PianoRollUiState;

/// Lowest visible pitch by default (C2).
pub const PITCH_MIN: i16 = 36;
/// Highest visible pitch by default (C6).
pub const PITCH_MAX: i16 = 84;
/// Horizontal pixels per beat at scale 1.
const PX_PER_BEAT: f32 = 80.0;
/// Vertical pixels per semitone at scale 1.
const PX_PER_SEMITONE: f32 = 18.0;
/// Width of the piano key lane in screen pixels.
const PIANO_WIDTH: f32 = 56.0;
/// Minimum canvas zoom.
const MIN_SCALE: f32 = 0.35;
/// Maximum canvas zoom.
const MAX_SCALE: f32 = 3.0;
/// Default visible duration in beats when empty.
const DEFAULT_BEATS: f64 = 8.0;
/// Screen-space hit width for note left/right resize edges.
const EDGE_HIT_PX: f32 = 6.0;
/// Fixed screen-space height of the playhead ruler strip above the grid.
pub const RULER_HEIGHT: f32 = 20.0;

/// Clamps canvas zoom.
pub fn clamp_scale(scale: f32) -> f32 {
    scale.clamp(MIN_SCALE, MAX_SCALE)
}

/// Grid / note edit intent published from the canvas to the host.
#[derive(Debug, Clone, PartialEq)]
pub enum PianoRollEdit {
    /// Place a note at snapped start/pitch with duration = snap size.
    Draw {
        /// Target document.
        document_id: DocumentId,
        /// Pitch.
        semitone: i16,
        /// Start in beats.
        start_beats: f64,
        /// Duration in beats.
        duration_beats: f64,
    },
    /// Select a note by index, or clear when `None`.
    Select {
        /// Target document.
        document_id: DocumentId,
        /// Note index in `events`, or `None` to clear.
        note_index: Option<usize>,
    },
    /// Move the selected note by beat/pitch deltas (already snapped).
    MoveSelected {
        /// Target document.
        document_id: DocumentId,
        /// Note index.
        note_index: usize,
        /// New start beats.
        start_beats: f64,
        /// New semitone.
        semitone: i16,
    },
    /// Resize the selected note (already snapped start/duration).
    ResizeSelected {
        /// Target document.
        document_id: DocumentId,
        /// Note index.
        note_index: usize,
        /// New start beats.
        start_beats: f64,
        /// New duration in beats.
        duration_beats: f64,
    },
    /// Delete a note by index.
    Erase {
        /// Target document.
        document_id: DocumentId,
        /// Note index.
        note_index: usize,
    },
    /// Park the playhead at a beat position (ruler click).
    SetPlayhead {
        /// Target document.
        document_id: DocumentId,
        /// Start position in beats.
        beats: f64,
    },
}

/// Which part of a note the cursor hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteHitPart {
    Body,
    LeftEdge,
    RightEdge,
}

/// Which horizontal edge is being resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeEdge {
    Left,
    Right,
}

/// Canvas program for the piano roll (time × pitch).
#[derive(Debug, Clone)]
pub struct PianoRollView {
    document_id: DocumentId,
    notation: Notation,
    tool: NotationTool,
    snap_enabled: bool,
    snap_resolution: SnapResolution,
    selected_note: Option<usize>,
    /// Playhead display position in beats (parked or live while playing).
    playhead_beats: f64,
}

impl PianoRollView {
    /// Creates a piano-roll view for the active notation document.
    pub fn new(
        document_id: DocumentId,
        notation: &Notation,
        ui: &PianoRollUiState,
        playhead_beats: f64,
    ) -> Self {
        Self {
            document_id,
            notation: notation.clone(),
            tool: ui.tool,
            snap_enabled: ui.snap_enabled,
            snap_resolution: ui.snap_resolution,
            selected_note: ui.selected_note,
            playhead_beats,
        }
    }

    fn content_beats(&self) -> f64 {
        self.notation
            .events
            .iter()
            .map(|note| note.start_beats + note.duration_beats)
            .fold(DEFAULT_BEATS, f64::max)
            .max(DEFAULT_BEATS)
    }

    fn world_from_cursor(&self, state: &GridState, local: Point) -> (f64, i16) {
        let x = (local.x - state.translation.x) / state.scale;
        let y = (local.y - RULER_HEIGHT - state.translation.y) / state.scale;
        let beats = (x / PX_PER_BEAT) as f64;
        let semitone = PITCH_MAX - (y / PX_PER_SEMITONE).floor() as i16;
        (beats.max(0.0), semitone.clamp(PITCH_MIN, PITCH_MAX))
    }

    fn beats_from_ruler_cursor(&self, state: &GridState, local: Point) -> f64 {
        let x = (local.x - state.translation.x) / state.scale;
        let beats = (x / PX_PER_BEAT) as f64;
        snap_beats(beats.max(0.0), self.snap_resolution.beats(), self.snap_enabled)
    }

    fn playhead_screen_x(&self, state: &GridState) -> f32 {
        state.translation.x + (self.playhead_beats as f32) * PX_PER_BEAT * state.scale
    }

    fn note_at(&self, state: &GridState, local: Point) -> Option<usize> {
        self.hit_note(state, local).map(|(index, _)| index)
    }

    fn hit_note(&self, state: &GridState, local: Point) -> Option<(usize, NoteHitPart)> {
        let (beats, semitone) = self.world_from_cursor(state, local);
        let edge_beats = f64::from(EDGE_HIT_PX / (state.scale.max(0.25) * PX_PER_BEAT));
        self.notation
            .events
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, note)| {
                if note.semitone != semitone {
                    return None;
                }
                let end = note.start_beats + note.duration_beats;
                if beats < note.start_beats || beats >= end {
                    return None;
                }
                let half = note.duration_beats * 0.5;
                let edge = edge_beats.min(half);
                let part = if beats < note.start_beats + edge {
                    NoteHitPart::LeftEdge
                } else if beats >= end - edge {
                    NoteHitPart::RightEdge
                } else {
                    NoteHitPart::Body
                };
                Some((index, part))
            })
    }

    fn min_duration_beats(&self) -> f64 {
        self.snap_resolution.beats()
    }
}

/// Pan/zoom and drag gesture state for the grid canvas.
#[derive(Debug, Clone)]
pub struct GridState {
    translation: Vector,
    scale: f32,
    /// Middle-button pan last point.
    pan_last: Option<Point>,
    /// Select-tool drag: note index and grab offset in beats.
    move_drag: Option<MoveDrag>,
    /// Select-tool edge resize drag.
    resize_drag: Option<ResizeDrag>,
    document_key: Option<u64>,
}

#[derive(Debug, Clone)]
struct MoveDrag {
    note_index: usize,
    grab_offset_beats: f64,
    origin_semitone: i16,
}

#[derive(Debug, Clone)]
struct ResizeDrag {
    note_index: usize,
    edge: ResizeEdge,
    origin_end_beats: f64,
    origin_start_beats: f64,
}

impl Default for GridState {
    fn default() -> Self {
        Self {
            translation: Vector::new(0.0, 0.0),
            scale: 1.0,
            pan_last: None,
            move_drag: None,
            resize_drag: None,
            document_key: None,
        }
    }
}

impl GridState {
    fn sync_document(&mut self, document_key: u64, bounds: Size) {
        if self.document_key == Some(document_key) {
            return;
        }
        self.document_key = Some(document_key);
        self.scale = 1.0;
        // Align top of C6 near the top of the viewport with a small margin.
        let top_y = 0.0;
        self.translation = Vector::new(8.0, 8.0 - top_y);
        let _ = bounds;
        self.pan_last = None;
        self.move_drag = None;
        self.resize_drag = None;
    }
}

impl<Message> canvas::Program<Message> for PianoRollView
where
    Message: 'static + Clone,
    Message: From<PianoRollEdit>,
{
    type State = GridState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        state.sync_document(self.document_id.get(), bounds.size());

        if let canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            if let (Some(last), Some(position)) = (state.pan_last, cursor.position_in(bounds)) {
                let delta = Vector::new(position.x - last.x, position.y - last.y);
                state.translation += delta;
                state.pan_last = Some(position);
                return Some(canvas::Action::request_redraw().and_capture());
            }
            if matches!(
                self.tool,
                NotationTool::Draw | NotationTool::Erase | NotationTool::Select
            ) {
                return Some(canvas::Action::request_redraw());
            }
            return None;
        }

        let cursor_in_bounds = cursor.position_in(bounds)?;

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.pan_last = Some(cursor_in_bounds);
                return Some(canvas::Action::capture());
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if state.pan_last.take().is_some() {
                    return Some(canvas::Action::capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor_in_bounds.y < RULER_HEIGHT {
                    let beats = self.beats_from_ruler_cursor(state, cursor_in_bounds);
                    let edit = PianoRollEdit::SetPlayhead {
                        document_id: self.document_id,
                        beats,
                    };
                    return Some(canvas::Action::publish(Message::from(edit)).and_capture());
                }
                match self.tool {
                    NotationTool::Draw => {
                        if self.note_at(state, cursor_in_bounds).is_some() {
                            return Some(canvas::Action::capture());
                        }
                        let (beats, semitone) = self.world_from_cursor(state, cursor_in_bounds);
                        let step = self.snap_resolution.beats();
                        let start = snap_beats(beats, step, self.snap_enabled);
                        let edit = PianoRollEdit::Draw {
                            document_id: self.document_id,
                            semitone,
                            start_beats: start,
                            duration_beats: step,
                        };
                        return Some(
                            canvas::Action::publish(Message::from(edit)).and_capture(),
                        );
                    }
                    NotationTool::Select => {
                        if let Some((index, part)) = self.hit_note(state, cursor_in_bounds) {
                            let note = &self.notation.events[index];
                            let (beats, _) = self.world_from_cursor(state, cursor_in_bounds);
                            state.move_drag = None;
                            state.resize_drag = None;
                            match part {
                                NoteHitPart::LeftEdge => {
                                    state.resize_drag = Some(ResizeDrag {
                                        note_index: index,
                                        edge: ResizeEdge::Left,
                                        origin_end_beats: note.start_beats + note.duration_beats,
                                        origin_start_beats: note.start_beats,
                                    });
                                }
                                NoteHitPart::RightEdge => {
                                    state.resize_drag = Some(ResizeDrag {
                                        note_index: index,
                                        edge: ResizeEdge::Right,
                                        origin_end_beats: note.start_beats + note.duration_beats,
                                        origin_start_beats: note.start_beats,
                                    });
                                }
                                NoteHitPart::Body => {
                                    state.move_drag = Some(MoveDrag {
                                        note_index: index,
                                        grab_offset_beats: beats - note.start_beats,
                                        origin_semitone: note.semitone,
                                    });
                                }
                            }
                            let edit = PianoRollEdit::Select {
                                document_id: self.document_id,
                                note_index: Some(index),
                            };
                            return Some(
                                canvas::Action::publish(Message::from(edit)).and_capture(),
                            );
                        }
                        state.move_drag = None;
                        state.resize_drag = None;
                        let edit = PianoRollEdit::Select {
                            document_id: self.document_id,
                            note_index: None,
                        };
                        return Some(
                            canvas::Action::publish(Message::from(edit)).and_capture(),
                        );
                    }
                    NotationTool::Erase => {
                        if let Some(index) = self.note_at(state, cursor_in_bounds) {
                            let edit = PianoRollEdit::Erase {
                                document_id: self.document_id,
                                note_index: index,
                            };
                            return Some(
                                canvas::Action::publish(Message::from(edit)).and_capture(),
                            );
                        }
                        return Some(canvas::Action::capture());
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(drag) = state.resize_drag.take() {
                    let (beats, _) = self.world_from_cursor(state, cursor_in_bounds);
                    let step = self.snap_resolution.beats();
                    let min_dur = self.min_duration_beats();
                    let (start_beats, duration_beats) = match drag.edge {
                        ResizeEdge::Right => {
                            let raw_end = beats.max(drag.origin_start_beats + min_dur);
                            let end = snap_beats(raw_end, step, self.snap_enabled)
                                .max(drag.origin_start_beats + min_dur);
                            (drag.origin_start_beats, end - drag.origin_start_beats)
                        }
                        ResizeEdge::Left => {
                            let max_start = (drag.origin_end_beats - min_dur).max(0.0);
                            let raw_start = beats.clamp(0.0, max_start);
                            let start = snap_beats(raw_start, step, self.snap_enabled)
                                .clamp(0.0, max_start);
                            (start, drag.origin_end_beats - start)
                        }
                    };
                    let edit = PianoRollEdit::ResizeSelected {
                        document_id: self.document_id,
                        note_index: drag.note_index,
                        start_beats,
                        duration_beats: duration_beats.max(min_dur),
                    };
                    return Some(canvas::Action::publish(Message::from(edit)).and_capture());
                }
                if let Some(drag) = state.move_drag.take() {
                    let (beats, semitone) = self.world_from_cursor(state, cursor_in_bounds);
                    let step = self.snap_resolution.beats();
                    let raw_start = (beats - drag.grab_offset_beats).max(0.0);
                    let start = snap_beats(raw_start, step, self.snap_enabled);
                    let _ = drag.origin_semitone;
                    let edit = PianoRollEdit::MoveSelected {
                        document_id: self.document_id,
                        note_index: drag.note_index,
                        start_beats: start,
                        semitone: semitone.clamp(PITCH_MIN, PITCH_MAX),
                    };
                    return Some(canvas::Action::publish(Message::from(edit)).and_capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                if y.abs() < f32::EPSILON {
                    return None;
                }
                let factor = if y > 0.0 { 1.1 } else { 1.0 / 1.1 };
                let new_scale = clamp_scale(state.scale * factor);
                if (new_scale - state.scale).abs() < f32::EPSILON {
                    return None;
                }
                let world = Point::new(
                    (cursor_in_bounds.x - state.translation.x) / state.scale,
                    (cursor_in_bounds.y - state.translation.y) / state.scale,
                );
                state.translation = Vector::new(
                    cursor_in_bounds.x - world.x * new_scale,
                    cursor_in_bounds.y - world.y * new_scale,
                );
                state.scale = new_scale;
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
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        let grid_color = palette.background.strong.color;
        let beat_color = palette.secondary.base.color;
        let note_fill = palette.primary.base.color;
        let note_selected = palette.primary.strong.color;
        let cursor_color = palette.background.base.text;
        let ruler_fill = palette.background.strong.color;
        let playhead_color = palette.danger.base.color;

        // Background
        frame.fill(
            &Path::rectangle(Point::ORIGIN, bounds.size()),
            palette.background.weak.color,
        );

        // Screen-space ruler strip (playhead placement chrome).
        frame.fill(
            &Path::rectangle(Point::ORIGIN, Size::new(bounds.width, RULER_HEIGHT)),
            ruler_fill,
        );

        frame.translate(Vector::new(state.translation.x, state.translation.y + RULER_HEIGHT));
        frame.scale(state.scale);

        let beats = self.content_beats();
        let step = self.snap_resolution.beats();
        let stroke_thin = Stroke::default()
            .with_width(1.0 / state.scale.max(0.25))
            .with_color(grid_color);
        let stroke_beat = Stroke::default()
            .with_width(1.5 / state.scale.max(0.25))
            .with_color(beat_color);

        // Horizontal pitch lines
        for semitone in PITCH_MIN..=PITCH_MAX {
            let y = f32::from(PITCH_MAX - semitone) * PX_PER_SEMITONE;
            let path = Path::line(
                Point::new(0.0, y),
                Point::new(beats as f32 * PX_PER_BEAT, y),
            );
            frame.stroke(&path, stroke_thin);
        }

        // Vertical snap / beat lines
        let mut t = 0.0;
        while t <= beats + step {
            let x = t as f32 * PX_PER_BEAT;
            let is_beat = (t / 1.0).fract().abs() < 1e-9 || (t % 1.0).abs() < 1e-9;
            let path = Path::line(
                Point::new(x, 0.0),
                Point::new(x, f32::from(PITCH_MAX - PITCH_MIN + 1) * PX_PER_SEMITONE),
            );
            frame.stroke(&path, if is_beat { stroke_beat } else { stroke_thin });
            t += step;
        }

        // Notes
        for (index, note) in self.notation.events.iter().enumerate() {
            if note.semitone < PITCH_MIN || note.semitone > PITCH_MAX {
                continue;
            }
            let x = note.start_beats as f32 * PX_PER_BEAT;
            let y = f32::from(PITCH_MAX - note.semitone) * PX_PER_SEMITONE;
            let w = (note.duration_beats as f32 * PX_PER_BEAT).max(4.0);
            let h = PX_PER_SEMITONE - 2.0;
            let rect = Path::rectangle(Point::new(x, y + 1.0), Size::new(w, h));
            let fill = if Some(index) == self.selected_note {
                note_selected
            } else {
                note_fill
            };
            frame.fill(&rect, fill);
        }

        let mut geometries = vec![frame.into_geometry()];

        // Playhead in screen space (through ruler + grid).
        {
            let mut playhead_frame = Frame::new(renderer, bounds.size());
            let x = self.playhead_screen_x(state);
            let stroke = Stroke::default().with_width(2.0).with_color(playhead_color);
            playhead_frame.stroke(
                &Path::line(Point::new(x, 0.0), Point::new(x, bounds.height)),
                stroke,
            );
            // Ruler marker triangle tip.
            let marker = Path::new(|builder| {
                builder.move_to(Point::new(x, RULER_HEIGHT));
                builder.line_to(Point::new(x - 5.0, 2.0));
                builder.line_to(Point::new(x + 5.0, 2.0));
                builder.close();
            });
            playhead_frame.fill(&marker, playhead_color);
            geometries.push(playhead_frame.into_geometry());
        }

        if state.pan_last.is_none() {
            if let Some(hotspot) = cursor.position_in(bounds) {
                if hotspot.y >= RULER_HEIGHT {
                    match self.tool {
                        NotationTool::Draw => {
                            geometries.push(draw_pen_cursor(
                                renderer,
                                bounds.size(),
                                hotspot,
                                cursor_color,
                            ));
                        }
                        NotationTool::Erase => {
                            geometries.push(draw_eraser_cursor(
                                renderer,
                                bounds.size(),
                                hotspot,
                                cursor_color,
                            ));
                        }
                        NotationTool::Select => {}
                    }
                }
            }
        }

        geometries
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.pan_last.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if !cursor.is_over(bounds) {
            return mouse::Interaction::default();
        }
        match self.tool {
            NotationTool::Draw | NotationTool::Erase => mouse::Interaction::Hidden,
            NotationTool::Select => {
                if state.resize_drag.is_some() {
                    mouse::Interaction::ResizingHorizontally
                } else if state.move_drag.is_some() {
                    mouse::Interaction::Grabbing
                } else if let Some(position) = cursor.position_in(bounds) {
                    match self.hit_note(state, position) {
                        Some((_, NoteHitPart::LeftEdge | NoteHitPart::RightEdge)) => {
                            mouse::Interaction::ResizingHorizontally
                        }
                        _ => mouse::Interaction::default(),
                    }
                } else {
                    mouse::Interaction::default()
                }
            }
        }
    }
}

/// Pen tip sits on `hotspot`; shaft extends below-right (NW bias for the body).
fn draw_pen_cursor(
    renderer: &Renderer,
    size: Size,
    hotspot: Point,
    color: Color,
) -> Geometry {
    let mut frame = Frame::new(renderer, size);
    let stroke = Stroke::default().with_width(1.5).with_color(color);
    // Tip at hotspot; body goes down-right.
    let tip = hotspot;
    let barrel_end = Point::new(hotspot.x + 11.0, hotspot.y + 11.0);
    let left = Point::new(hotspot.x + 3.0, hotspot.y + 7.0);
    let right = Point::new(hotspot.x + 7.0, hotspot.y + 3.0);
    let nib = Path::new(|builder| {
        builder.move_to(tip);
        builder.line_to(left);
        builder.line_to(right);
        builder.close();
    });
    frame.fill(&nib, color);
    frame.stroke(
        &Path::line(Point::new(hotspot.x + 5.0, hotspot.y + 5.0), barrel_end),
        stroke,
    );
    frame.stroke(
        &Path::line(
            Point::new(barrel_end.x - 2.0, barrel_end.y + 2.0),
            Point::new(barrel_end.x + 2.0, barrel_end.y - 2.0),
        ),
        stroke,
    );
    frame.into_geometry()
}

/// Eraser working corner sits on `hotspot`; body extends below-right.
fn draw_eraser_cursor(
    renderer: &Renderer,
    size: Size,
    hotspot: Point,
    color: Color,
) -> Geometry {
    let mut frame = Frame::new(renderer, size);
    let stroke = Stroke::default().with_width(1.5).with_color(color);
    // Parallelogram: top-left corner = hotspot (working edge).
    let p0 = hotspot;
    let p1 = Point::new(hotspot.x + 12.0, hotspot.y + 2.0);
    let p2 = Point::new(hotspot.x + 16.0, hotspot.y + 10.0);
    let p3 = Point::new(hotspot.x + 4.0, hotspot.y + 8.0);
    let eraser = Path::new(|builder| {
        builder.move_to(p0);
        builder.line_to(p1);
        builder.line_to(p2);
        builder.line_to(p3);
        builder.close();
    });
    frame.stroke(&eraser, stroke);
    // Band line across the block.
    frame.stroke(
        &Path::line(
            Point::new(hotspot.x + 3.0, hotspot.y + 5.0),
            Point::new(hotspot.x + 13.0, hotspot.y + 7.0),
        ),
        stroke,
    );
    frame.into_geometry()
}

/// Piano key lane program (labels + black/white keys).
#[derive(Debug, Clone)]
pub struct PianoLaneView {
    translation_y: f32,
    scale: f32,
}

impl Default for PianoLaneView {
    fn default() -> Self {
        Self {
            translation_y: 8.0,
            scale: 1.0,
        }
    }
}

impl PianoLaneView {
    fn new(translation_y: f32, scale: f32) -> Self {
        Self {
            translation_y,
            scale,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PianoLaneState;

impl<Message> canvas::Program<Message> for PianoLaneView {
    type State = PianoLaneState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        let white = palette.background.base.color;
        let black = palette.background.strong.color;
        let border = palette.background.strong.color;
        let label = palette.background.base.text;

        frame.fill(
            &Path::rectangle(Point::ORIGIN, bounds.size()),
            palette.background.weak.color,
        );

        frame.translate(Vector::new(0.0, self.translation_y));
        frame.scale(self.scale);

        let stroke = Stroke::default()
            .with_width(1.0 / self.scale.max(0.25))
            .with_color(border);

        for semitone in (PITCH_MIN..=PITCH_MAX).rev() {
            let y = f32::from(PITCH_MAX - semitone) * PX_PER_SEMITONE;
            let fill = if is_black_key(semitone) { black } else { white };
            let rect = Path::rectangle(Point::new(0.0, y), Size::new(PIANO_WIDTH, PX_PER_SEMITONE));
            frame.fill(&rect, fill);
            frame.stroke(&rect, stroke);
            frame.fill_text(Text {
                content: semitone_label(semitone),
                position: Point::new(4.0, y + 2.0),
                color: if is_black_key(semitone) {
                    white
                } else {
                    label
                },
                size: Pixels(11.0),
                font: Font::DEFAULT,
                ..Text::default()
            });
        }

        vec![frame.into_geometry()]
    }
}

/// Builds the piano-roll panel: piano lane + grid.
pub fn piano_roll_panel<'a, Message>(
    document_id: DocumentId,
    notation: &'a Notation,
    ui: &PianoRollUiState,
    playhead_beats: f64,
) -> Element<'a, Message>
where
    Message: 'static + Clone + From<PianoRollEdit>,
{
    let grid = Canvas::new(PianoRollView::new(
        document_id,
        notation,
        ui,
        playhead_beats,
    ))
    .width(Fill)
    .height(Fill);

    // Piano lane top margin matches grid translation below the ruler strip.
    let piano = Canvas::new(PianoLaneView::new(8.0 + RULER_HEIGHT, 1.0))
        .width(PIANO_WIDTH)
        .height(Fill);

    row![piano, grid].spacing(0).width(Fill).height(Fill).into()
}

/// Applies a draw/move/erase edit to notation; returns whether it changed.
pub fn apply_edit(notation: &mut Notation, edit: &PianoRollEdit) -> bool {
    match edit {
        PianoRollEdit::Draw {
            semitone,
            start_beats,
            duration_beats,
            ..
        } => {
            // Avoid stacking identical notes on the same cell.
            if notation.events.iter().any(|note| {
                note.semitone == *semitone
                    && (note.start_beats - start_beats).abs() < 1e-9
                    && (note.duration_beats - duration_beats).abs() < 1e-9
            }) {
                return false;
            }
            notation
                .events
                .push(NotationNote::new(*semitone, *start_beats, *duration_beats));
            true
        }
        PianoRollEdit::Select { .. } | PianoRollEdit::SetPlayhead { .. } => false,
        PianoRollEdit::MoveSelected {
            note_index,
            start_beats,
            semitone,
            ..
        } => {
            if let Some(note) = notation.events.get_mut(*note_index) {
                if (note.start_beats - start_beats).abs() < 1e-9 && note.semitone == *semitone {
                    return false;
                }
                note.start_beats = *start_beats;
                note.semitone = *semitone;
                true
            } else {
                false
            }
        }
        PianoRollEdit::ResizeSelected {
            note_index,
            start_beats,
            duration_beats,
            ..
        } => {
            if let Some(note) = notation.events.get_mut(*note_index) {
                if (note.start_beats - start_beats).abs() < 1e-9
                    && (note.duration_beats - duration_beats).abs() < 1e-9
                {
                    return false;
                }
                note.start_beats = *start_beats;
                note.duration_beats = *duration_beats;
                true
            } else {
                false
            }
        }
        PianoRollEdit::Erase { note_index, .. } => {
            if *note_index < notation.events.len() {
                notation.events.remove(*note_index);
                true
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piano_roll_ui::PianoRollUiState;

    #[test]
    fn draw_edit_appends_note() {
        let mut notation = Notation::empty();
        let edit = PianoRollEdit::Draw {
            document_id: DocumentId::from_raw(1),
            semitone: 60,
            start_beats: 0.0,
            duration_beats: 0.25,
        };
        assert!(apply_edit(&mut notation, &edit));
        assert_eq!(notation.events.len(), 1);
        assert_eq!(notation.events[0].semitone, 60);
    }

    #[test]
    fn erase_removes_note() {
        let mut notation = Notation::empty();
        notation.events.push(NotationNote::new(60, 0.0, 0.25));
        let edit = PianoRollEdit::Erase {
            document_id: DocumentId::from_raw(1),
            note_index: 0,
        };
        assert!(apply_edit(&mut notation, &edit));
        assert!(notation.events.is_empty());
    }

    #[test]
    fn piano_roll_panel_builds() {
        let notation = Notation::empty();
        let ui = PianoRollUiState::default();
        let _: Element<'_, PianoRollEdit> =
            piano_roll_panel(DocumentId::from_raw(1), &notation, &ui, 0.0);
    }
}
