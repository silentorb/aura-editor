//! User-state component registration contracts.

/// Metadata for a registered user-state implementation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserStateDescriptor {
    /// Optional human-readable label.
    pub label: Option<String>,
}

impl UserStateDescriptor {
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

/// Host surface that accepts user-state implementations from extensions.
pub trait UserStateHost {
    /// Registers a user-state implementation under `implementation_id`.
    fn register_user_state(&mut self, implementation_id: &str, descriptor: UserStateDescriptor);
}
