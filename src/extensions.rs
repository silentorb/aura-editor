//! Host extension runtime: load config, collect registrations, build registries.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use aura_interfaces::{
    parse_extensions_json, AuraJsonEnvelope, CommandDescriptor, CommandHost, ComponentKind,
    DocumentOpenError, DocumentType, DocumentTypeHost, ExtensionConfig, ExtensionsConfig,
    OpenSource, ViewDescriptor, ViewHost, DOC_TYPE_AURA_JSON,
};
use tracing::{info, warn};

/// Bundled default `extensions.json`.
pub const DEFAULT_EXTENSIONS_JSON: &str =
    include_str!("../assets/default-extensions.json");

/// Successful stacked open: chain general→specific plus tip payload.
#[derive(Debug)]
pub struct StackedOpen {
    /// Document-type ids from root to tip.
    pub type_chain: Vec<String>,
    /// Parallel optional view roles for each chain entry.
    pub roles: Vec<Option<String>>,
    /// Tip payload produced by the most specific type.
    pub payload: Box<dyn Any + Send + Sync>,
}

/// Resolved command, view, and document-type registries after config + registration.
#[derive(Clone, Default)]
pub struct ExtensionRuntime {
    commands: HashSet<String>,
    /// role → view implementation id (first enabled claim wins).
    views_by_role: HashMap<String, String>,
    /// implementation id → registered (for diagnostics).
    view_implementations: HashSet<String>,
    /// Enabled document types by implementation id.
    document_types: HashMap<String, Arc<dyn DocumentType>>,
    /// Root document-type ids in config order.
    root_document_types: Vec<String>,
    /// parent id → child implementation ids in config order.
    document_type_children: HashMap<String, Vec<String>>,
}

impl std::fmt::Debug for ExtensionRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtensionRuntime")
            .field("commands", &self.commands)
            .field("views_by_role", &self.views_by_role)
            .field("view_implementations", &self.view_implementations)
            .field(
                "document_types",
                &self.document_types.keys().collect::<Vec<_>>(),
            )
            .field("root_document_types", &self.root_document_types)
            .field("document_type_children", &self.document_type_children)
            .finish()
    }
}

impl PartialEq for ExtensionRuntime {
    fn eq(&self, other: &Self) -> bool {
        self.commands == other.commands
            && self.views_by_role == other.views_by_role
            && self.view_implementations == other.view_implementations
            && self.root_document_types == other.root_document_types
            && self.document_type_children == other.document_type_children
            && self.document_types.keys().collect::<HashSet<_>>()
                == other.document_types.keys().collect::<HashSet<_>>()
    }
}

impl Eq for ExtensionRuntime {}

impl ExtensionRuntime {
    /// Empty registries (no commands, views, or document types).
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
            document_types = runtime.document_types.len(),
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

    /// Whether a document type implementation is enabled.
    pub fn has_document_type(&self, implementation_id: &str) -> bool {
        self.document_types.contains_key(implementation_id)
    }

    /// Read `path` and resolve the stacked document-type chain.
    pub fn open_document(&self, path: impl AsRef<Path>) -> Result<StackedOpen, DocumentOpenError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|err| DocumentOpenError::Io {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        self.open_document_bytes(path, &bytes)
    }

    /// Resolve stacked document types from already-read bytes.
    pub fn open_document_bytes(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<StackedOpen, DocumentOpenError> {
        let source = OpenSource { path, bytes };

        let mut type_chain = Vec::new();
        let mut roles = Vec::new();

        let root_id = self
            .root_document_types
            .iter()
            .find(|id| {
                self.document_types
                    .get(id.as_str())
                    .is_some_and(|dt| dt.matches_root(&source))
            })
            .cloned()
            .ok_or_else(|| DocumentOpenError::Unrecognized {
                path: path.to_path_buf(),
            })?;

        let root = self.document_types.get(&root_id).expect("root registered");
        let mut payload = root.open_root(&source)?;
        type_chain.push(root_id.clone());
        roles.push(root.role().map(str::to_owned));

        let mut current_id = root_id;
        while let Some(child_ids) = self.document_type_children.get(&current_id) {
            if child_ids.is_empty() {
                break;
            }

            let child_id = if current_id == DOC_TYPE_AURA_JSON {
                let envelope = payload.downcast_ref::<AuraJsonEnvelope>().ok_or_else(|| {
                    DocumentOpenError::Invalid {
                        path: path.to_path_buf(),
                        reason: String::from("aura-json tip payload missing envelope"),
                    }
                })?;
                let type_id = envelope.type_id.clone();
                if !child_ids.iter().any(|id| id == &type_id) {
                    return Err(DocumentOpenError::UnknownAuraType {
                        path: path.to_path_buf(),
                        type_id,
                    });
                }
                type_id
            } else {
                match child_ids.iter().find(|id| {
                    self.document_types
                        .get(id.as_str())
                        .is_some_and(|dt| dt.matches_child(payload.as_ref()))
                }) {
                    Some(id) => id.clone(),
                    None => break,
                }
            };

            let child = self
                .document_types
                .get(&child_id)
                .expect("child registered");
            payload = child.open_child(path, payload.as_ref())?;
            type_chain.push(child_id.clone());
            roles.push(child.role().map(str::to_owned));
            current_id = child_id;
        }

        Ok(StackedOpen {
            type_chain,
            roles,
            payload,
        })
    }

    /// Walk roles from tip to root; return the first role that has an enabled view.
    pub fn resolve_view_for_chain<'a>(
        &'a self,
        roles: &'a [Option<String>],
    ) -> Option<(&'a str, &'a str)> {
        for role in roles.iter().rev().flatten() {
            if let Some(view_id) = self.view_for_role(role) {
                return Some((role.as_str(), view_id));
            }
        }
        None
    }
}

/// Collects offered implementations before config filtering.
pub struct RegistrationHost {
    config: ExtensionsConfig,
    offered_commands: HashMap<String, CommandDescriptor>,
    offered_views: HashMap<String, ViewDescriptor>,
    offered_document_types: HashMap<String, Arc<dyn DocumentType>>,
}

impl std::fmt::Debug for RegistrationHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistrationHost")
            .field("config", &self.config)
            .field("offered_commands", &self.offered_commands)
            .field("offered_views", &self.offered_views)
            .field(
                "offered_document_types",
                &self.offered_document_types.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl RegistrationHost {
    fn new(config: ExtensionsConfig) -> Self {
        Self {
            config,
            offered_commands: HashMap::new(),
            offered_views: HashMap::new(),
            offered_document_types: HashMap::new(),
        }
    }

    fn extension(&self, id: &str) -> Option<&ExtensionConfig> {
        self.config.extensions.iter().find(|ext| ext.id == id)
    }

    fn finish(self) -> ExtensionRuntime {
        let mut commands = HashSet::new();
        let mut views_by_role = HashMap::new();
        let mut view_implementations = HashSet::new();
        let mut document_types = HashMap::new();
        let mut root_document_types = Vec::new();
        let mut document_type_children: HashMap<String, Vec<String>> = HashMap::new();
        let mut seen_roots = HashSet::new();
        let mut seen_children: HashMap<String, HashSet<String>> = HashMap::new();

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
                ComponentKind::DocumentType => {
                    let Some(document_type) =
                        self.offered_document_types.get(&component.implementation_id)
                    else {
                        warn!(
                            component = %component.id,
                            implementation = %component.implementation_id,
                            "skipping document-type; implementation not registered by a linked extension"
                        );
                        continue;
                    };
                    let id = component.implementation_id.clone();
                    document_types.insert(id.clone(), Arc::clone(document_type));
                    match document_type.parent_id() {
                        None => {
                            if seen_roots.insert(id.clone()) {
                                root_document_types.push(id);
                            }
                        }
                        Some(parent) => {
                            let kids = seen_children.entry(parent.to_string()).or_default();
                            if kids.insert(id.clone()) {
                                document_type_children
                                    .entry(parent.to_string())
                                    .or_default()
                                    .push(id);
                            }
                        }
                    }
                }
            }
        }

        ExtensionRuntime {
            commands,
            views_by_role,
            view_implementations,
            document_types,
            root_document_types,
            document_type_children,
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

impl DocumentTypeHost for RegistrationHost {
    fn register_document_type(
        &mut self,
        implementation_id: &str,
        document_type: Arc<dyn DocumentType>,
    ) {
        self.offered_document_types
            .insert(implementation_id.to_string(), document_type);
    }
}

/// Calls `register` on every compile-time linked extension package.
pub fn register_linked_extensions(host: &mut RegistrationHost) {
    aura_ext_builtin::register(host);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use aura_interfaces::{
        CMD_DOCUMENT_OPEN, DOC_TYPE_AURA_JSON, DOC_TYPE_IMP_GRAPH, DOC_TYPE_IMP_GRAPH_LIBRARY,
        DOC_TYPE_JSON, ROLE_IMP_GRAPH, VIEW_GRAPH_DAG,
    };

    #[test]
    fn bundled_config_enables_builtin_commands_views_and_document_types() {
        let runtime = ExtensionRuntime::load_bundled();
        assert!(runtime.has_command(CMD_DOCUMENT_OPEN));
        assert!(runtime.has_command("document.close_active"));
        assert_eq!(runtime.view_for_role(ROLE_IMP_GRAPH), Some(VIEW_GRAPH_DAG));
        assert!(runtime.has_document_type(DOC_TYPE_JSON));
        assert!(runtime.has_document_type(DOC_TYPE_AURA_JSON));
        assert!(runtime.has_document_type(DOC_TYPE_IMP_GRAPH));
        assert!(runtime.has_document_type(DOC_TYPE_IMP_GRAPH_LIBRARY));
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
            },
            {
              "id": "json",
              "extensionId": "aura-builtin",
              "kind": "document-type",
              "implementationId": "json",
              "enabled": true
            },
            {
              "id": "aura-json",
              "extensionId": "aura-builtin",
              "kind": "document-type",
              "implementationId": "aura-json",
              "enabled": true
            },
            {
              "id": "imp-graph",
              "extensionId": "aura-builtin",
              "kind": "document-type",
              "implementationId": "imp-graph",
              "enabled": true
            }
          ]
        }"#;
        let runtime =
            ExtensionRuntime::load_from_json(json, register_linked_extensions);
        assert!(!runtime.has_command(CMD_DOCUMENT_OPEN));
        assert_eq!(runtime.view_for_role(ROLE_IMP_GRAPH), Some(VIEW_GRAPH_DAG));
        assert!(runtime.has_document_type(DOC_TYPE_IMP_GRAPH));
    }

    #[test]
    fn missing_view_role_returns_none() {
        let runtime = ExtensionRuntime::empty();
        assert!(runtime.view_for_role(ROLE_IMP_GRAPH).is_none());
    }

    #[test]
    fn open_sine_fixture_stacks_to_imp_graph() {
        let runtime = ExtensionRuntime::load_bundled();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sine.json");
        let opened = runtime.open_document(&path).expect("open sine");
        assert_eq!(
            opened.type_chain,
            vec![
                DOC_TYPE_JSON.to_string(),
                DOC_TYPE_AURA_JSON.to_string(),
                DOC_TYPE_IMP_GRAPH.to_string()
            ]
        );
        assert!(opened.payload.downcast_ref::<imp_core_types::Graph>().is_some());
    }

    #[test]
    fn open_instruments_rejects_graph_library() {
        let runtime = ExtensionRuntime::load_bundled();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/instruments.json");
        let err = runtime.open_document(&path).unwrap_err();
        assert!(matches!(
            err,
            DocumentOpenError::GraphLibraryNotSupported { .. }
        ));
    }
}
