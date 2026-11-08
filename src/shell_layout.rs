//! Dockable shell pane-grid layout (documents / properties).

use std::collections::HashSet;

use iced::widget::pane_grid::{self, Axis, Configuration, Node, Pane};
use serde::{Deserialize, Serialize};

/// Stable ids for the shell body panes.
///
/// [`Status`](Self::Status) and [`ViewToolbar`](Self::ViewToolbar) remain on the enum so older
/// user-state trees deserialize for migration; they are not live dockable panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellPanelId {
    /// Former global status / error line (migrated out of the pane grid).
    Status,
    /// Former active-view tool strip (now Documents chrome; migrated out of the pane grid).
    ViewToolbar,
    /// Document tabs + view toolbar + content pane.
    Documents,
    /// Contextual properties inspector (graph node Id/Type in this slice).
    Properties,
}

impl ShellPanelId {
    /// Title-bar label shown in the pane grid.
    pub fn title(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::ViewToolbar => "Toolbar",
            Self::Documents => "Documents",
            Self::Properties => "Properties",
        }
    }

    /// Live dockable panel ids in a stable order.
    pub fn all() -> [Self; 2] {
        [Self::Documents, Self::Properties]
    }

    /// True when this id is stripped during migration to the current layout version.
    fn is_legacy_chrome(self) -> bool {
        matches!(self, Self::Status | Self::ViewToolbar)
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

/// Supported `ShellLayoutEntry::version` (after migration).
pub const SHELL_LAYOUT_ENTRY_VERSION: u32 = 3;

/// Prior three-pane layout version; migrated by inserting Properties then stripping chrome panes.
const SHELL_LAYOUT_ENTRY_VERSION_V1: u32 = 1;

/// Prior four-pane layout version; migrated by stripping status and view-toolbar.
const SHELL_LAYOUT_ENTRY_VERSION_V2: u32 = 2;

/// Default documents share of the documents|properties vertical split.
const DEFAULT_DOCUMENTS_RATIO: f32 = 0.75;

/// Former default status split ratio (used only when reconstructing legacy test trees).
#[cfg(test)]
const DEFAULT_STATUS_RATIO: f32 = 0.08;

/// Former default toolbar split ratio (used only when reconstructing legacy test trees).
#[cfg(test)]
const DEFAULT_TOOLBAR_RATIO: f32 = 0.10;

/// Builds the default pane-grid configuration (documents | properties).
pub fn default_configuration() -> Configuration<ShellPanelId> {
    Configuration::Split {
        axis: Axis::Vertical,
        ratio: DEFAULT_DOCUMENTS_RATIO,
        a: Box::new(Configuration::Pane(ShellPanelId::Documents)),
        b: Box::new(Configuration::Pane(ShellPanelId::Properties)),
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

    /// Validates the tree (migrating older versions when needed) and builds pane-grid state, or [`None`].
    pub fn try_into_state(&self) -> Option<pane_grid::State<ShellPanelId>> {
        let migrated = self.migrated()?;
        if !migrated.tree.is_valid() {
            return None;
        }
        Some(pane_grid::State::with_configuration(
            migrated.tree.to_configuration(),
        ))
    }

    /// Like [`Self::try_into_state`], falling back to [`default_state`] on failure.
    pub fn into_state_or_default(&self) -> pane_grid::State<ShellPanelId> {
        self.try_into_state().unwrap_or_else(default_state)
    }

    /// Returns a v3 entry, migrating valid v1/v2 trees when needed.
    fn migrated(&self) -> Option<Self> {
        match self.version {
            SHELL_LAYOUT_ENTRY_VERSION => Some(self.clone()),
            SHELL_LAYOUT_ENTRY_VERSION_V2 => {
                let tree = self.tree.strip_legacy_chrome()?;
                Some(Self {
                    version: SHELL_LAYOUT_ENTRY_VERSION,
                    tree,
                })
            }
            SHELL_LAYOUT_ENTRY_VERSION_V1 => {
                let tree = self.tree.migrate_v1_insert_properties()?;
                let tree = tree.strip_legacy_chrome()?;
                Some(Self {
                    version: SHELL_LAYOUT_ENTRY_VERSION,
                    tree,
                })
            }
            _ => None,
        }
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

    /// True when the tree contains each live [`ShellPanelId`] exactly once and ratios are finite.
    pub fn is_valid(&self) -> bool {
        let mut seen = HashSet::new();
        if !self.collect_panels(&mut seen) {
            return false;
        }
        seen.len() == ShellPanelId::all().len()
            && ShellPanelId::all().iter().all(|id| seen.contains(id))
            && !seen.iter().any(|id| id.is_legacy_chrome())
    }

    /// True when the tree is a valid v1 layout (status, view-toolbar, documents exactly once).
    fn is_valid_v1(&self) -> bool {
        let mut seen = HashSet::new();
        if !self.collect_panels(&mut seen) {
            return false;
        }
        let v1 = [
            ShellPanelId::Status,
            ShellPanelId::ViewToolbar,
            ShellPanelId::Documents,
        ];
        seen.len() == v1.len()
            && v1.iter().all(|id| seen.contains(id))
            && !seen.contains(&ShellPanelId::Properties)
    }

    /// True when the tree is a valid v2 layout (status, view-toolbar, documents, properties).
    fn is_valid_v2(&self) -> bool {
        let mut seen = HashSet::new();
        if !self.collect_panels(&mut seen) {
            return false;
        }
        let v2 = [
            ShellPanelId::Status,
            ShellPanelId::ViewToolbar,
            ShellPanelId::Documents,
            ShellPanelId::Properties,
        ];
        seen.len() == v2.len() && v2.iter().all(|id| seen.contains(id))
    }

    /// Rewrites a v1 tree by replacing each `documents` leaf with documents|properties.
    fn migrate_v1_insert_properties(&self) -> Option<Self> {
        if !self.is_valid_v1() {
            return None;
        }
        Some(self.replace_documents_with_docs_properties())
    }

    /// Removes status / view-toolbar leaves and collapses parent splits onto the surviving sibling.
    fn strip_legacy_chrome(&self) -> Option<Self> {
        if self.is_valid() {
            return Some(self.clone());
        }
        if !self.is_valid_v2() {
            return None;
        }
        self.strip_legacy_chrome_inner()
    }

    fn strip_legacy_chrome_inner(&self) -> Option<Self> {
        match self {
            Self::Pane(id) if id.is_legacy_chrome() => None,
            Self::Pane(id) => Some(Self::Pane(*id)),
            Self::Split {
                axis,
                ratio,
                a,
                b,
            } => match (
                a.strip_legacy_chrome_inner(),
                b.strip_legacy_chrome_inner(),
            ) {
                (None, None) => None,
                (Some(kept), None) | (None, Some(kept)) => Some(kept),
                (Some(a), Some(b)) => Some(Self::Split {
                    axis: *axis,
                    ratio: *ratio,
                    a: Box::new(a),
                    b: Box::new(b),
                }),
            },
        }
    }

    fn replace_documents_with_docs_properties(&self) -> Self {
        match self {
            Self::Pane(ShellPanelId::Documents) => Self::Split {
                axis: ShellAxis::Vertical,
                ratio: DEFAULT_DOCUMENTS_RATIO,
                a: Box::new(Self::Pane(ShellPanelId::Documents)),
                b: Box::new(Self::Pane(ShellPanelId::Properties)),
            },
            Self::Pane(id) => Self::Pane(*id),
            Self::Split {
                axis,
                ratio,
                a,
                b,
            } => Self::Split {
                axis: *axis,
                ratio: *ratio,
                a: Box::new(a.replace_documents_with_docs_properties()),
                b: Box::new(b.replace_documents_with_docs_properties()),
            },
        }
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

    fn v1_default_tree() -> ShellLayoutTree {
        ShellLayoutTree::Split {
            axis: ShellAxis::Horizontal,
            ratio: DEFAULT_STATUS_RATIO,
            a: Box::new(ShellLayoutTree::Pane(ShellPanelId::Status)),
            b: Box::new(ShellLayoutTree::Split {
                axis: ShellAxis::Horizontal,
                ratio: DEFAULT_TOOLBAR_RATIO,
                a: Box::new(ShellLayoutTree::Pane(ShellPanelId::ViewToolbar)),
                b: Box::new(ShellLayoutTree::Pane(ShellPanelId::Documents)),
            }),
        }
    }

    fn v2_default_tree() -> ShellLayoutTree {
        ShellLayoutTree::Split {
            axis: ShellAxis::Horizontal,
            ratio: DEFAULT_STATUS_RATIO,
            a: Box::new(ShellLayoutTree::Pane(ShellPanelId::Status)),
            b: Box::new(ShellLayoutTree::Split {
                axis: ShellAxis::Horizontal,
                ratio: DEFAULT_TOOLBAR_RATIO,
                a: Box::new(ShellLayoutTree::Pane(ShellPanelId::ViewToolbar)),
                b: Box::new(ShellLayoutTree::Split {
                    axis: ShellAxis::Vertical,
                    ratio: DEFAULT_DOCUMENTS_RATIO,
                    a: Box::new(ShellLayoutTree::Pane(ShellPanelId::Documents)),
                    b: Box::new(ShellLayoutTree::Pane(ShellPanelId::Properties)),
                }),
            }),
        }
    }

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
        assert_eq!(restored.version, SHELL_LAYOUT_ENTRY_VERSION);
    }

    #[test]
    fn invalid_missing_panel_falls_back() {
        let entry = ShellLayoutEntry {
            version: SHELL_LAYOUT_ENTRY_VERSION,
            tree: ShellLayoutTree::Pane(ShellPanelId::Documents),
        };
        assert!(entry.try_into_state().is_none());
        let state = entry.into_state_or_default();
        let panels: HashSet<_> = state.iter().map(|(_, id)| *id).collect();
        assert_eq!(panels.len(), 2);
        assert!(panels.contains(&ShellPanelId::Documents));
        assert!(panels.contains(&ShellPanelId::Properties));
    }

    #[test]
    fn duplicate_panel_is_invalid() {
        let entry = ShellLayoutEntry {
            version: SHELL_LAYOUT_ENTRY_VERSION,
            tree: ShellLayoutTree::Split {
                axis: ShellAxis::Horizontal,
                ratio: 0.5,
                a: Box::new(ShellLayoutTree::Pane(ShellPanelId::Documents)),
                b: Box::new(ShellLayoutTree::Pane(ShellPanelId::Documents)),
            },
        };
        assert!(!entry.tree.is_valid());
    }

    #[test]
    fn unsupported_version_rejected() {
        let entry = ShellLayoutEntry {
            version: 99,
            ..ShellLayoutEntry::default()
        };
        assert!(entry.try_into_state().is_none());
    }

    #[test]
    fn v1_three_pane_tree_migrates_to_documents_properties() {
        let entry = ShellLayoutEntry {
            version: SHELL_LAYOUT_ENTRY_VERSION_V1,
            tree: v1_default_tree(),
        };
        let state = entry.try_into_state().expect("v1 should migrate");
        let panels: HashSet<_> = state.iter().map(|(_, id)| *id).collect();
        assert_eq!(panels.len(), 2);
        assert!(panels.contains(&ShellPanelId::Documents));
        assert!(panels.contains(&ShellPanelId::Properties));
        assert!(!panels.contains(&ShellPanelId::Status));
        assert!(!panels.contains(&ShellPanelId::ViewToolbar));

        let restored = ShellLayoutEntry::from_state(&state).expect("serialize migrated");
        assert_eq!(
            restored.tree,
            ShellLayoutEntry::default().tree,
            "default v1 stack should migrate to the v3 default"
        );
    }

    #[test]
    fn v2_four_pane_tree_migrates_to_documents_properties() {
        let entry = ShellLayoutEntry {
            version: SHELL_LAYOUT_ENTRY_VERSION_V2,
            tree: v2_default_tree(),
        };
        let state = entry.try_into_state().expect("v2 should migrate");
        let panels: HashSet<_> = state.iter().map(|(_, id)| *id).collect();
        assert_eq!(panels.len(), 2);
        assert!(panels.contains(&ShellPanelId::Documents));
        assert!(panels.contains(&ShellPanelId::Properties));

        let restored = ShellLayoutEntry::from_state(&state).expect("serialize migrated");
        assert_eq!(
            restored.tree,
            ShellLayoutEntry::default().tree,
            "default v2 stack should migrate to the v3 default"
        );
    }

    #[test]
    fn entry_json_matches_documented_shape() {
        let entry = ShellLayoutEntry::default();
        let value = serde_json::to_value(&entry).expect("serialize");
        assert_eq!(value["version"], 3);
        assert_eq!(value["tree"]["split"]["axis"], "vertical");
        assert_eq!(value["tree"]["split"]["a"]["pane"], "documents");
        assert_eq!(value["tree"]["split"]["b"]["pane"], "properties");
        assert_eq!(value["tree"]["split"]["ratio"], 0.75);
    }
}
