//! Dockable shell pane-grid layout (status / view-toolbar / documents).

use std::collections::HashSet;

use iced::widget::pane_grid::{self, Axis, Configuration, Node, Pane};
use serde::{Deserialize, Serialize};

/// Stable ids for the three shell body panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellPanelId {
    /// Global status / error line.
    Status,
    /// Active view tool strip (empty when no tools apply).
    ViewToolbar,
    /// Document tabs + content pane.
    Documents,
}

impl ShellPanelId {
    /// Title-bar label shown in the pane grid.
    pub fn title(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::ViewToolbar => "Toolbar",
            Self::Documents => "Documents",
        }
    }

    /// All panel ids in a stable order.
    pub fn all() -> [Self; 3] {
        [Self::Status, Self::ViewToolbar, Self::Documents]
    }
}

/// Versioned user-state entry for the shell pane grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellLayoutEntry {
    /// Entry schema version.
    pub version: u32,
    /// Recursive layout tree.
    pub tree: ShellLayoutTree,
}

impl Default for ShellLayoutEntry {
    fn default() -> Self {
        Self::from_configuration(&default_configuration())
    }
}

/// Recursive layout tree mirroring [`Configuration`] without iced types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellLayoutTree {
    /// A leaf pane.
    #[serde(rename = "pane")]
    Pane(ShellPanelId),
    /// A binary split of space.
    #[serde(rename = "split")]
    Split {
        /// Split direction (`horizontal` = top/bottom, `vertical` = left/right).
        axis: ShellAxis,
        /// Ratio of the first child in `[0.0, 1.0]`.
        ratio: f32,
        /// Left / top subtree.
        a: Box<ShellLayoutTree>,
        /// Right / bottom subtree.
        b: Box<ShellLayoutTree>,
    },
}

/// Serializable pane-grid axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellAxis {
    /// Top / bottom split (horizontal divider).
    Horizontal,
    /// Left / right split (vertical divider).
    Vertical,
}

impl From<ShellAxis> for Axis {
    fn from(value: ShellAxis) -> Self {
        match value {
            ShellAxis::Horizontal => Axis::Horizontal,
            ShellAxis::Vertical => Axis::Vertical,
        }
    }
}

impl From<Axis> for ShellAxis {
    fn from(value: Axis) -> Self {
        match value {
            Axis::Horizontal => ShellAxis::Horizontal,
            Axis::Vertical => ShellAxis::Vertical,
        }
    }
}

/// Supported `ShellLayoutEntry::version`.
pub const SHELL_LAYOUT_ENTRY_VERSION: u32 = 1;

/// Default status split ratio (top strip).
const DEFAULT_STATUS_RATIO: f32 = 0.08;

/// Default toolbar split ratio within the remaining body.
const DEFAULT_TOOLBAR_RATIO: f32 = 0.10;

/// Builds the default pane-grid configuration (status → toolbar → documents).
pub fn default_configuration() -> Configuration<ShellPanelId> {
    Configuration::Split {
        axis: Axis::Horizontal,
        ratio: DEFAULT_STATUS_RATIO,
        a: Box::new(Configuration::Pane(ShellPanelId::Status)),
        b: Box::new(Configuration::Split {
            axis: Axis::Horizontal,
            ratio: DEFAULT_TOOLBAR_RATIO,
            a: Box::new(Configuration::Pane(ShellPanelId::ViewToolbar)),
            b: Box::new(Configuration::Pane(ShellPanelId::Documents)),
        }),
    }
}

/// Creates pane-grid state from the default layout.
pub fn default_state() -> pane_grid::State<ShellPanelId> {
    pane_grid::State::with_configuration(default_configuration())
}

impl ShellLayoutEntry {
    /// Builds an entry from a live pane-grid configuration tree.
    pub fn from_configuration(config: &Configuration<ShellPanelId>) -> Self {
        Self {
            version: SHELL_LAYOUT_ENTRY_VERSION,
            tree: ShellLayoutTree::from_configuration(config),
        }
    }

    /// Builds an entry by walking live pane-grid state.
    pub fn from_state(state: &pane_grid::State<ShellPanelId>) -> Option<Self> {
        let tree = ShellLayoutTree::from_node(state.layout(), &state.panes)?;
        Some(Self {
            version: SHELL_LAYOUT_ENTRY_VERSION,
            tree,
        })
    }

    /// Validates the tree and builds pane-grid state, or returns [`None`] when invalid.
    pub fn try_into_state(&self) -> Option<pane_grid::State<ShellPanelId>> {
        if self.version != SHELL_LAYOUT_ENTRY_VERSION {
            return None;
        }
        if !self.tree.is_valid() {
            return None;
        }
        Some(pane_grid::State::with_configuration(
            self.tree.to_configuration(),
        ))
    }

    /// Like [`Self::try_into_state`], falling back to [`default_state`] on failure.
    pub fn into_state_or_default(&self) -> pane_grid::State<ShellPanelId> {
        self.try_into_state().unwrap_or_else(default_state)
    }
}

impl ShellLayoutTree {
    fn from_configuration(config: &Configuration<ShellPanelId>) -> Self {
        match config {
            Configuration::Pane(id) => Self::Pane(*id),
            Configuration::Split {
                axis,
                ratio,
                a,
                b,
            } => Self::Split {
                axis: (*axis).into(),
                ratio: *ratio,
                a: Box::new(Self::from_configuration(a)),
                b: Box::new(Self::from_configuration(b)),
            },
        }
    }

    fn from_node(
        node: &Node,
        panes: &std::collections::BTreeMap<Pane, ShellPanelId>,
    ) -> Option<Self> {
        match node {
            Node::Pane(pane) => panes.get(pane).copied().map(Self::Pane),
            Node::Split {
                axis,
                ratio,
                a,
                b,
                ..
            } => Some(Self::Split {
                axis: (*axis).into(),
                ratio: *ratio,
                a: Box::new(Self::from_node(a, panes)?),
                b: Box::new(Self::from_node(b, panes)?),
            }),
        }
    }

    fn to_configuration(&self) -> Configuration<ShellPanelId> {
        match self {
            Self::Pane(id) => Configuration::Pane(*id),
            Self::Split {
                axis,
                ratio,
                a,
                b,
            } => Configuration::Split {
                axis: (*axis).into(),
                ratio: *ratio,
                a: Box::new(a.to_configuration()),
                b: Box::new(b.to_configuration()),
            },
        }
    }

    /// True when the tree contains each [`ShellPanelId`] exactly once and ratios are finite.
    pub fn is_valid(&self) -> bool {
        let mut seen = HashSet::new();
        if !self.collect_panels(&mut seen) {
            return false;
        }
        seen.len() == ShellPanelId::all().len()
            && ShellPanelId::all().iter().all(|id| seen.contains(id))
    }

    fn collect_panels(&self, seen: &mut HashSet<ShellPanelId>) -> bool {
        match self {
            Self::Pane(id) => seen.insert(*id),
            Self::Split {
                ratio, a, b, ..
            } => {
                ratio.is_finite()
                    && (0.0..=1.0).contains(ratio)
                    && a.collect_panels(seen)
                    && b.collect_panels(seen)
            }
        }
    }
}

/// Restores pane-grid state from a user-state entry, or the default layout.
pub fn state_from_entry(entry: Option<&ShellLayoutEntry>) -> pane_grid::State<ShellPanelId> {
    match entry {
        Some(entry) => entry.into_state_or_default(),
        None => default_state(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tree_is_valid() {
        let entry = ShellLayoutEntry::default();
        assert_eq!(entry.version, SHELL_LAYOUT_ENTRY_VERSION);
        assert!(entry.tree.is_valid());
        assert!(entry.try_into_state().is_some());
    }

    #[test]
    fn round_trip_entry_through_state() {
        let original = ShellLayoutEntry::default();
        let state = original.try_into_state().expect("default should load");
        let restored = ShellLayoutEntry::from_state(&state).expect("state should serialize");
        assert_eq!(restored.tree, original.tree);
    }

    #[test]
    fn invalid_missing_panel_falls_back() {
        let entry = ShellLayoutEntry {
            version: SHELL_LAYOUT_ENTRY_VERSION,
            tree: ShellLayoutTree::Pane(ShellPanelId::Status),
        };
        assert!(entry.try_into_state().is_none());
        let state = entry.into_state_or_default();
        let panels: HashSet<_> = state.iter().map(|(_, id)| *id).collect();
        assert_eq!(panels.len(), 3);
    }

    #[test]
    fn duplicate_panel_is_invalid() {
        let entry = ShellLayoutEntry {
            version: SHELL_LAYOUT_ENTRY_VERSION,
            tree: ShellLayoutTree::Split {
                axis: ShellAxis::Horizontal,
                ratio: 0.5,
                a: Box::new(ShellLayoutTree::Pane(ShellPanelId::Status)),
                b: Box::new(ShellLayoutTree::Pane(ShellPanelId::Status)),
            },
        };
        assert!(!entry.tree.is_valid());
    }

    #[test]
    fn unsupported_version_rejected() {
        let mut entry = ShellLayoutEntry::default();
        entry.version = 99;
        assert!(entry.try_into_state().is_none());
    }

    #[test]
    fn entry_json_matches_documented_shape() {
        let entry = ShellLayoutEntry::default();
        let value = serde_json::to_value(&entry).expect("serialize");
        assert_eq!(value["version"], 1);
        assert_eq!(value["tree"]["split"]["axis"], "horizontal");
        assert_eq!(value["tree"]["split"]["a"]["pane"], "status");
        assert_eq!(
            value["tree"]["split"]["b"]["split"]["a"]["pane"],
            "view-toolbar"
        );
        assert_eq!(
            value["tree"]["split"]["b"]["split"]["b"]["pane"],
            "documents"
        );
    }
}
