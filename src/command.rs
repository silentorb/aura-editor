//! Stable editor command identifiers (string ids from the extension registry).

use std::fmt;

/// A bindable editor command id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandId(String);

impl CommandId {
    /// Creates a command id from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Stable string id used in bindings JSON and the command registry.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for CommandId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CommandId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
