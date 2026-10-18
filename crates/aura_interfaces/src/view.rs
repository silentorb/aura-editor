//! View component registration contracts.

/// Metadata for a registered view implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewDescriptor {
    /// Document roles this implementation can present when config omits `roles`.
    pub default_roles: Vec<String>,
}

impl ViewDescriptor {
    /// Descriptor with the given default roles.
    pub fn with_roles(roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            default_roles: roles.into_iter().map(Into::into).collect(),
        }
    }
}

/// Host surface that accepts view implementations from extensions.
pub trait ViewHost {
    /// Registers a view implementation under `implementation_id`.
    fn register_view(&mut self, implementation_id: &str, descriptor: ViewDescriptor);
}
