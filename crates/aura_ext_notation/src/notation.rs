//! Serde model for Aura JSON `notation` documents.

use std::fmt;

use serde::{Deserialize, Serialize};

/// On-disk / in-memory notation document (Aura JSON tip `notation`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notation {
    /// Aura JSON document-type id; must be `notation`.
    #[serde(rename = "type")]
    pub type_id: String,
    /// Schema version (currently 1).
    pub version: u32,
    /// Tempo in beats per minute.
    pub tempo_bpm: f64,
    /// Meter for the notation.
    pub time_signature: NotationTimeSignature,
    /// Discrete note events on the beat grid.
    pub events: Vec<NotationNote>,
}

impl Default for Notation {
    fn default() -> Self {
        Self::empty()
    }
}

impl Notation {
    /// Empty notation document with default tempo and 4/4 meter.
    pub fn empty() -> Self {
        Self {
            type_id: aura_interfaces::DOC_TYPE_NOTATION.to_string(),
            version: 1,
            tempo_bpm: 120.0,
            time_signature: NotationTimeSignature::FOUR_FOUR,
            events: Vec::new(),
        }
    }

    /// Serializes this document as pretty Aura JSON.
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parses a notation document from a JSON value (full Aura JSON object).
    pub fn from_value(value: serde_json::Value) -> Result<Self, String> {
        let notation: Self =
            serde_json::from_value(value).map_err(|err| err.to_string())?;
        if notation.type_id != aura_interfaces::DOC_TYPE_NOTATION {
            return Err(format!(
                "expected type \"{}\", got \"{}\"",
                aura_interfaces::DOC_TYPE_NOTATION,
                notation.type_id
            ));
        }
        if notation.version != 1 {
            return Err(format!("unsupported notation version {}", notation.version));
        }
        Ok(notation)
    }
}

/// Beats per bar and beat unit (e.g. 4/4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotationTimeSignature {
    /// Numerator.
    pub beats_per_bar: u8,
    /// Denominator (note value that gets the beat).
    pub beat_unit: u8,
}

impl NotationTimeSignature {
    /// Common-time meter.
    pub const FOUR_FOUR: Self = Self {
        beats_per_bar: 4,
        beat_unit: 4,
    };
}

/// One note event on the beat × semitone grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotationNote {
    /// Pitch as semitones above C0 (scientific; middle C = 60 = C4).
    pub semitone: i16,
    /// Start time in quarter-note beats.
    pub start_beats: f64,
    /// Duration in quarter-note beats.
    pub duration_beats: f64,
    /// Velocity in `0.0..=1.0` (MIDI-like intensity).
    #[serde(default = "default_velocity")]
    pub velocity: f32,
}

fn default_velocity() -> f32 {
    1.0
}

impl NotationNote {
    /// Creates a note with full velocity.
    pub fn new(semitone: i16, start_beats: f64, duration_beats: f64) -> Self {
        Self {
            semitone,
            start_beats,
            duration_beats,
            velocity: 1.0,
        }
    }
}

/// Snap subdivision relative to one beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SnapResolution {
    /// Quarter of a beat.
    #[serde(rename = "1/4")]
    Quarter,
    /// Eighth of a beat.
    #[serde(rename = "1/8")]
    Eighth,
    /// Sixteenth of a beat.
    #[serde(rename = "1/16")]
    #[default]
    Sixteenth,
    /// Thirty-second of a beat.
    #[serde(rename = "1/32")]
    ThirtySecond,
}

impl SnapResolution {
    /// All selectable resolutions in display order.
    pub const ALL: [Self; 4] = [
        Self::Quarter,
        Self::Eighth,
        Self::Sixteenth,
        Self::ThirtySecond,
    ];

    /// Size of one snap unit in beats.
    pub fn beats(self) -> f64 {
        match self {
            Self::Quarter => 0.25,
            Self::Eighth => 0.125,
            Self::Sixteenth => 0.0625,
            Self::ThirtySecond => 0.03125,
        }
    }

    /// Label for UI and persistence.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quarter => "1/4",
            Self::Eighth => "1/8",
            Self::Sixteenth => "1/16",
            Self::ThirtySecond => "1/32",
        }
    }

    /// Parses a resolution label.
    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "1/4" => Some(Self::Quarter),
            "1/8" => Some(Self::Eighth),
            "1/16" => Some(Self::Sixteenth),
            "1/32" => Some(Self::ThirtySecond),
            _ => None,
        }
    }
}

impl fmt::Display for SnapResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Active notation editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotationTool {
    /// Places notes on click.
    #[default]
    Draw,
    /// Selects and moves notes.
    Select,
    /// Deletes notes on click.
    Erase,
}

impl NotationTool {
    /// Stable id for persistence and commands.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draw => "draw",
            Self::Select => "select",
            Self::Erase => "erase",
        }
    }

    /// Parses a tool id.
    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "draw" => Some(Self::Draw),
            "select" => Some(Self::Select),
            "erase" => Some(Self::Erase),
            _ => None,
        }
    }
}

/// Snaps `value` down to the nearest multiple of `step` when snap is enabled.
pub fn snap_beats(value: f64, step: f64, enabled: bool) -> f64 {
    if !enabled || step <= 0.0 || !value.is_finite() {
        return value.max(0.0);
    }
    let snapped = (value / step).floor() * step;
    snapped.max(0.0)
}

/// Scientific pitch label for a MIDI-compatible semitone (60 → `C4`).
pub fn semitone_label(semitone: i16) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pitch_class = semitone.rem_euclid(12) as usize;
    let octave = (i32::from(semitone) / 12) - 1;
    format!("{}{}", NAMES[pitch_class], octave)
}

/// Whether `semitone` is a black key in 12-TET (relative to C).
pub fn is_black_key(semitone: i16) -> bool {
    matches!(semitone.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_empty_notation_json() {
        let notation = Notation::empty();
        let json = notation.to_pretty_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let parsed = Notation::from_value(value).unwrap();
        assert_eq!(parsed, notation);
    }

    #[test]
    fn semitone_label_middle_c() {
        assert_eq!(semitone_label(60), "C4");
        assert_eq!(semitone_label(61), "C#4");
        assert_eq!(semitone_label(69), "A4");
    }

    #[test]
    fn snap_beats_floors_to_grid() {
        assert_eq!(snap_beats(0.3, 0.25, true), 0.25);
        assert_eq!(snap_beats(0.3, 0.25, false), 0.3);
    }
}
