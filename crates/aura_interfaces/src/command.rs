//! Command component registration contracts.

/// Metadata for a registered command implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDescriptor {
    /// Optional human-readable label.
    pub label: Option<String>,
}

impl CommandDescriptor {
    /// Descriptor with no label.
    pub fn empty() -> Self {
        Self { label: None }
    }

    /// Descriptor with a label.
    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
        }
    }
}

/// Host surface that accepts command implementations from extensions.
pub trait CommandHost {
    /// Registers a command implementation under `implementation_id`.
    fn register_command(&mut self, implementation_id: &str, descriptor: CommandDescriptor);
}
