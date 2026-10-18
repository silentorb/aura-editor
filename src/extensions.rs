//! Host extension runtime: load config, collect registrations, build registries.

use std::collections::{HashMap, HashSet};

use aura_interfaces::{
    parse_extensions_json, CommandDescriptor, CommandHost, ComponentKind, ExtensionConfig,
    ExtensionsConfig, ViewDescriptor, ViewHost,
};
use tracing::{info, warn};

/// Bundled default `extensions.json`.
pub const DEFAULT_EXTENSIONS_JSON: &str =
    include_str!("../assets/default-extensions.json");

/// Resolved command and view registries after config + registration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionRuntime {
    commands: HashSet<String>,
    /// role → view implementation id (first enabled claim wins).
    views_by_role: HashMap<String, String>,
    /// implementation id → registered (for diagnostics).
    view_implementations: HashSet<String>,
}

impl ExtensionRuntime {
    /// Empty registries (no commands or views).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load bundled default config and register linked builtin extension.
    pub fn load_bundled() -> Self {
        Self::load_from_json(DEFAULT_EXTENSIONS_JSON, register_linked_extensions)
    }

    /// Parse config JSON and apply the linked-extension registration callback.
    pub fn load_from_json(
        json: &str,
        register_linked: impl FnOnce(&mut RegistrationHost),
    ) -> Self {
        let config = match parse_extensions_json(json) {
            Ok(config) => config,
            Err(error) => {
                warn!(%error, "failed to load extensions config; using empty registries");
                return Self::empty();
            }
        };
        Self::build(config, register_linked)
    }

    /// Build registries from an already-parsed config.
    pub fn build(
        config: ExtensionsConfig,
        register_linked: impl FnOnce(&mut RegistrationHost),
    ) -> Self {
        let mut host = RegistrationHost::new(config);
        register_linked(&mut host);
        let runtime = host.finish();
        info!(
            commands = runtime.commands.len(),
            views = runtime.view_implementations.len(),
            roles = runtime.views_by_role.len(),
            "extension registries ready"
        );
        runtime
    }

    /// Whether `command_id` is enabled and registered.
    pub fn has_command(&self, command_id: &str) -> bool {
        self.commands.contains(command_id)
    }

    /// Enabled command ids.
    pub fn command_ids(&self) -> &HashSet<String> {
        &self.commands
    }

    /// View implementation id bound to `role`, if any.
    pub fn view_for_role(&self, role: &str) -> Option<&str> {
        self.views_by_role.get(role).map(String::as_str)
    }

    /// Whether any view implementation was activated.
    pub fn has_view_implementation(&self, implementation_id: &str) -> bool {
        self.view_implementations.contains(implementation_id)
    }
}

/// Collects offered implementations before config filtering.
#[derive(Debug)]
pub struct RegistrationHost {
    config: ExtensionsConfig,
    offered_commands: HashMap<String, CommandDescriptor>,
    offered_views: HashMap<String, ViewDescriptor>,
}

impl RegistrationHost {
    fn new(config: ExtensionsConfig) -> Self {
        Self {
            config,
            offered_commands: HashMap::new(),
            offered_views: HashMap::new(),
        }
    }

    fn extension(&self, id: &str) -> Option<&ExtensionConfig> {
        self.config.extensions.iter().find(|ext| ext.id == id)
    }

    fn finish(self) -> ExtensionRuntime {
        let mut commands = HashSet::new();
        let mut views_by_role = HashMap::new();
        let mut view_implementations = HashSet::new();

        for component in &self.config.components {
            if !component.enabled {
                continue;
            }
            let Some(extension) = self.extension(&component.extension_id) else {
                warn!(
                    component = %component.id,
                    extension = %component.extension_id,
                    "skipping component; extension id not listed"
                );
                continue;
            };
            if !extension.enabled {
                continue;
            }

            match component.kind {
                ComponentKind::Command => {
                    if !self
                        .offered_commands
                        .contains_key(&component.implementation_id)
                    {
                        warn!(
                            component = %component.id,
                            implementation = %component.implementation_id,
                            "skipping command; implementation not registered by a linked extension"
                        );
                        continue;
                    }
                    // Command component id is the bindable string id.
                    commands.insert(component.id.clone());
                }
                ComponentKind::View => {
                    let Some(descriptor) =
                        self.offered_views.get(&component.implementation_id)
                    else {
                        warn!(
                            component = %component.id,
                            implementation = %component.implementation_id,
                            "skipping view; implementation not registered by a linked extension"
                        );
                        continue;
                    };
                    let roles = if component.roles.is_empty() {
                        if descriptor.default_roles.is_empty() {
                            vec![component.id.clone()]
                        } else {
                            descriptor.default_roles.clone()
                        }
                    } else {
                        component.roles.clone()
                    };
                    view_implementations.insert(component.implementation_id.clone());
                    for role in roles {
                        views_by_role
                            .entry(role)
                            .or_insert_with(|| component.implementation_id.clone());
                    }
                }
            }
        }

        ExtensionRuntime {
            commands,
            views_by_role,
            view_implementations,
        }
    }
}

impl CommandHost for RegistrationHost {
    fn register_command(&mut self, implementation_id: &str, descriptor: CommandDescriptor) {
        self.offered_commands
            .insert(implementation_id.to_string(), descriptor);
    }
}

impl ViewHost for RegistrationHost {
    fn register_view(&mut self, implementation_id: &str, descriptor: ViewDescriptor) {
        self.offered_views
            .insert(implementation_id.to_string(), descriptor);
    }
}

/// Calls `register` on every compile-time linked extension package.
pub fn register_linked_extensions(host: &mut RegistrationHost) {
    aura_ext_builtin::register(host);
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_interfaces::{CMD_DOCUMENT_OPEN, ROLE_IMP_GRAPH, VIEW_GRAPH_DAG};

    #[test]
    fn bundled_config_enables_builtin_commands_and_view() {
        let runtime = ExtensionRuntime::load_bundled();
        assert!(runtime.has_command(CMD_DOCUMENT_OPEN));
        assert!(runtime.has_command("document.close_active"));
        assert_eq!(runtime.view_for_role(ROLE_IMP_GRAPH), Some(VIEW_GRAPH_DAG));
    }

    #[test]
    fn disabled_component_is_omitted() {
        let json = r#"{
          "version": 1,
          "extensions": [{ "id": "aura-builtin", "enabled": true }],
          "components": [
            {
              "id": "document.open",
              "extensionId": "aura-builtin",
              "kind": "command",
              "implementationId": "document.open",
              "enabled": false
            },
            {
              "id": "graph-dag",
              "extensionId": "aura-builtin",
              "kind": "view",
              "implementationId": "graph-dag",
              "roles": ["imp-graph"],
              "enabled": true
            }
          ]
        }"#;
        let runtime =
            ExtensionRuntime::load_from_json(json, register_linked_extensions);
        assert!(!runtime.has_command(CMD_DOCUMENT_OPEN));
        assert_eq!(runtime.view_for_role(ROLE_IMP_GRAPH), Some(VIEW_GRAPH_DAG));
    }

    #[test]
    fn missing_view_role_returns_none() {
        let runtime = ExtensionRuntime::empty();
        assert!(runtime.view_for_role(ROLE_IMP_GRAPH).is_none());
    }
}
