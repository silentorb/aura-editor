//! Stable editor command identifiers.

/// A bindable editor command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    /// Open a graph file via the native dialog.
    DocumentOpen,
    /// Close the active document tab, if any.
    DocumentCloseActive,
}

impl CommandId {
    /// Stable string id used in bindings JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentOpen => "document.open",
            Self::DocumentCloseActive => "document.close_active",
        }
    }

    /// Parses a known command id. Unknown ids return `None`.
    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "document.open" => Some(Self::DocumentOpen),
            "document.close_active" => Some(Self::DocumentCloseActive),
            _ => None,
        }
    }
}
