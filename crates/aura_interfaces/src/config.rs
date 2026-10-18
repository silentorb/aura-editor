//! `extensions.json` schema and parsing.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

/// Supported `extensions.json` schema version.
pub const SUPPORTED_VERSION: u32 = 1;

/// Top-level extensions configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExtensionsConfig {
    /// Schema version.
    pub version: u32,
    /// Linked extension packages.
    #[serde(default)]
    pub extensions: Vec<ExtensionConfig>,
    /// Component instances wired into the host.
    #[serde(default)]
    pub components: Vec<ComponentConfig>,
}

/// One extension package row.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExtensionConfig {
    /// Stable extension id (e.g. `aura-builtin`).
    pub id: String,
    /// Whether the extension is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Extension-level params merged under component params.
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
}

/// One component instance row.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ComponentConfig {
    /// Stable component id (often equal to `implementation_id`).
    pub id: String,
    /// Owning extension id.
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    /// Component kind.
    pub kind: ComponentKind,
    /// Implementation id registered by the extension package.
    #[serde(rename = "implementationId")]
    pub implementation_id: String,
    /// Whether this component is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional human label.
    #[serde(default)]
    pub label: Option<String>,
    /// Roles this component fills (views; optional for commands).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Component-level params (win over extension params on key conflict).
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
}

/// Known component kinds for v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentKind {
    /// Bindable editor command.
    Command,
    /// Document content view.
    View,
}

fn default_true() -> bool {
    true
}

/// Failure while parsing or validating extensions config.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExtensionsError {
    /// JSON did not parse.
    #[error("invalid extensions JSON: {0}")]
    Json(String),
    /// Unsupported schema version.
    #[error("unsupported extensions version {found} (expected {SUPPORTED_VERSION})")]
    UnsupportedVersion {
        /// Version found in the file.
        found: u32,
    },
}

/// Parse and validate an `extensions.json` document.
pub fn parse_extensions_json(json: &str) -> Result<ExtensionsConfig, ExtensionsError> {
    let config: ExtensionsConfig =
        serde_json::from_str(json).map_err(|e| ExtensionsError::Json(e.to_string()))?;
    if config.version != SUPPORTED_VERSION {
        return Err(ExtensionsError::UnsupportedVersion {
            found: config.version,
        });
    }
    Ok(config)
}

impl ComponentConfig {
    /// Merged params: extension params first, then component params.
    pub fn merged_params(
        &self,
        extension: Option<&ExtensionConfig>,
    ) -> BTreeMap<String, Value> {
        let mut merged = BTreeMap::new();
        if let Some(ext) = extension {
            merged.extend(ext.params.clone());
        }
        merged.extend(self.params.clone());
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let config = parse_extensions_json(
            r#"{
              "version": 1,
              "extensions": [{ "id": "aura-builtin", "enabled": true }],
              "components": [{
                "id": "document.open",
                "extensionId": "aura-builtin",
                "kind": "command",
                "implementationId": "document.open",
                "enabled": true
              }]
            }"#,
        )
        .unwrap();
        assert_eq!(config.extensions.len(), 1);
        assert_eq!(config.components[0].kind, ComponentKind::Command);
    }

    #[test]
    fn rejects_unsupported_version() {
        let err = parse_extensions_json(r#"{"version": 99}"#).unwrap_err();
        assert!(matches!(
            err,
            ExtensionsError::UnsupportedVersion { found: 99 }
        ));
    }

    #[test]
    fn merges_params_component_wins() {
        let ext = ExtensionConfig {
            id: "x".into(),
            enabled: true,
            params: BTreeMap::from([("a".into(), Value::from(1)), ("b".into(), Value::from(2))]),
        };
        let component = ComponentConfig {
            id: "c".into(),
            extension_id: "x".into(),
            kind: ComponentKind::Command,
            implementation_id: "c".into(),
            enabled: true,
            label: None,
            roles: vec![],
            params: BTreeMap::from([("b".into(), Value::from(9))]),
        };
        let merged = component.merged_params(Some(&ext));
        assert_eq!(merged.get("a"), Some(&Value::from(1)));
        assert_eq!(merged.get("b"), Some(&Value::from(9)));
    }
}
