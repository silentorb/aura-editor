//! Persisted piano-roll tool / snap preferences.

use aura_ext_notation::{NotationTool, SnapResolution};
use serde::{Deserialize, Serialize};

/// User-state entry for piano-roll UI prefs (`piano-roll-ui`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PianoRollUiEntry {
    /// Active modal tool id.
    pub active_tool: String,
    /// Whether snap-to-grid is enabled.
    pub snap_enabled: bool,
    /// Snap subdivision label (`1/4` … `1/32`).
    pub snap_resolution: String,
}

impl Default for PianoRollUiEntry {
    fn default() -> Self {
        Self {
            active_tool: NotationTool::Draw.as_str().to_string(),
            snap_enabled: true,
            snap_resolution: SnapResolution::Sixteenth.as_str().to_string(),
        }
    }
}

/// In-memory piano-roll UI state used by the host.
#[derive(Debug, Clone, PartialEq)]
pub struct PianoRollUiState {
    /// Active modal tool.
    pub tool: NotationTool,
    /// Snap enabled.
    pub snap_enabled: bool,
    /// Snap resolution.
    pub snap_resolution: SnapResolution,
    /// Selected note index in the active notation, if any.
    pub selected_note: Option<usize>,
    /// Parked playhead start position in beats (session-only; snaps back here on stop).
    pub playhead_beats: f64,
}

impl Default for PianoRollUiState {
    fn default() -> Self {
        Self::from_entry(&PianoRollUiEntry::default())
    }
}

impl PianoRollUiState {
    /// Builds state from a persisted entry (invalid fields fall back to defaults).
    pub fn from_entry(entry: &PianoRollUiEntry) -> Self {
        Self {
            tool: NotationTool::parse(&entry.active_tool).unwrap_or_default(),
            snap_enabled: entry.snap_enabled,
            snap_resolution: SnapResolution::parse(&entry.snap_resolution).unwrap_or_default(),
            selected_note: None,
            playhead_beats: 0.0,
        }
    }

    /// Serializes prefs for user-state persistence (selection/playhead are session-only).
    pub fn to_entry(&self) -> PianoRollUiEntry {
        PianoRollUiEntry {
            active_tool: self.tool.as_str().to_string(),
            snap_enabled: self.snap_enabled,
            snap_resolution: self.snap_resolution.as_str().to_string(),
        }
    }
}
