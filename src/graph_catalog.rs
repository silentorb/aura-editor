//! Adapts Aura / Imp registries to [`imp_graph_editor::NodeTypeCatalog`].

use aura_imp::{get_node_type, Registry};
use imp_graph_editor::{CatalogPort, NodeTypeCatalog};

/// Thin wrapper so Aura's `Registry` satisfies the graph-editor catalog trait.
pub struct RegistryCatalog<'a>(pub &'a Registry);

impl NodeTypeCatalog for RegistryCatalog<'_> {
    fn inputs(&self, node_type_id: &str) -> Option<Vec<CatalogPort>> {
        let def = get_node_type(self.0, node_type_id)?;
        Some(
            def.inputs
                .values()
                .map(|port| CatalogPort {
                    id: port.id.clone(),
                    // Serialize via Aura's Imp types (may differ from editor path dep).
                    signal_key: serde_json::to_string(&port.signal_type).unwrap_or_default(),
                })
                .collect(),
        )
    }

    fn outputs(&self, node_type_id: &str) -> Option<Vec<CatalogPort>> {
        let def = get_node_type(self.0, node_type_id)?;
        Some(
            def.outputs
                .values()
                .map(|port| CatalogPort {
                    id: port.id.clone(),
                    signal_key: serde_json::to_string(&port.signal_type).unwrap_or_default(),
                })
                .collect(),
        )
    }
}
