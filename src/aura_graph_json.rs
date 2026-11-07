//! Aura JSON serialization for Imp graph documents.

use imp_core_types::Graph;

/// Serializes an Imp graph as pretty Aura JSON with `type: "imp-graph"`.
pub fn graph_to_aura_json(graph: &Graph) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(graph)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.insert(
            "type".into(),
            serde_json::Value::String(String::from("imp-graph")),
        );
    }
    serde_json::to_string_pretty(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_graph_editor::add_node;
    use std::collections::BTreeMap;

    #[test]
    fn graph_to_aura_json_sets_type() {
        let mut graph = Graph {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
        };
        add_node(&mut graph, "out", "output").unwrap();
        let json = graph_to_aura_json(&graph).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "imp-graph");
        assert!(value.get("nodes").is_some());
    }
}
