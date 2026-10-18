//! Configurable input→command bindings loaded from JSON.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use iced::keyboard::{key::Named, Key, Modifiers};
use serde::Deserialize;
use tracing::{info, warn};

use crate::command::CommandId;

/// Bundled default bindings JSON.
pub const DEFAULT_BINDINGS_JSON: &str =
    include_str!("../assets/default-bindings.json");

const SUPPORTED_VERSION: u32 = 1;

/// Runtime bindings table: input trigger → command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bindings {
    key_map: HashMap<KeyBinding, CommandId>,
}

/// A keyboard trigger used for lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct KeyBinding {
    key: BindingKey,
    modifiers: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BindingKey {
    Character(String),
    Named(Named),
}

#[derive(Debug, Deserialize)]
struct BindingsFile {
    version: u32,
    #[serde(default)]
    bindings: Vec<BindingEntry>,
}

#[derive(Debug, Deserialize)]
struct BindingEntry {
    command: String,
    when: TriggerFile,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TriggerFile {
    Key {
        key: String,
        #[serde(default)]
        modifiers: Vec<String>,
    },
}

impl Bindings {
    /// Empty table (no input bindings).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Load using the production chain against `known_commands`.
    pub fn load(known_commands: &HashSet<String>) -> Self {
        let user_path = user_bindings_path();
        Self::load_chain(user_path.as_deref(), DEFAULT_BINDINGS_JSON, known_commands)
    }

    /// Load with an explicit user path (tests) and a default JSON string.
    pub fn load_chain(
        user_path: Option<&Path>,
        default_json: &str,
        known_commands: &HashSet<String>,
    ) -> Self {
        if let Some(path) = user_path {
            if path.is_file() {
                match fs::read_to_string(path) {
                    Ok(contents) => match Self::from_json_str(&contents, known_commands) {
                        Ok(bindings) => {
                            info!(
                                path = %path.display(),
                                count = bindings.len(),
                                "loaded user input bindings"
                            );
                            return bindings;
                        }
                        Err(error) => {
                            warn!(
                                path = %path.display(),
                                %error,
                                "failed to load user input bindings; trying default"
                            );
                        }
                    },
                    Err(error) => {
                        warn!(
                            path = %path.display(),
                            %error,
                            "failed to read user input bindings; trying default"
                        );
                    }
                }
            }
        }

        match Self::from_json_str(default_json, known_commands) {
            Ok(bindings) => {
                info!(
                    count = bindings.len(),
                    "loaded bundled default input bindings"
                );
                bindings
            }
            Err(error) => {
                warn!(
                    %error,
                    "failed to load bundled default input bindings; using empty"
                );
                Self::empty()
            }
        }
    }

    /// Parse bindings from a JSON string, skipping ids not in `known_commands`.
    pub fn from_json_str(
        json: &str,
        known_commands: &HashSet<String>,
    ) -> Result<Self, String> {
        let file: BindingsFile =
            serde_json::from_str(json).map_err(|e| e.to_string())?;
        if file.version != SUPPORTED_VERSION {
            return Err(format!(
                "unsupported bindings version {} (expected {SUPPORTED_VERSION})",
                file.version
            ));
        }

        let mut key_map = HashMap::new();
        for entry in file.bindings {
            if !known_commands.contains(&entry.command) {
                warn!(
                    command = %entry.command,
                    "skipping unknown command id in bindings"
                );
                continue;
            }
            let command = CommandId::new(entry.command.clone());

            match entry.when {
                TriggerFile::Key { key, modifiers } => {
                    let binding = match parse_key_binding(&key, &modifiers) {
                        Ok(binding) => binding,
                        Err(error) => {
                            warn!(
                                command = %entry.command,
                                %error,
                                "skipping invalid key binding"
                            );
                            continue;
                        }
                    };
                    if key_map.insert(binding, command).is_some() {
                        warn!(
                            command = %entry.command,
                            "duplicate input trigger; last binding wins"
                        );
                    }
                }
            }
        }

        Ok(Self { key_map })
    }

    /// Number of resolved bindings.
    pub fn len(&self) -> usize {
        self.key_map.len()
    }

    /// Whether the table has no bindings.
    pub fn is_empty(&self) -> bool {
        self.key_map.is_empty()
    }

    /// Resolve a keyboard event to a command, if bound.
    pub fn resolve(
        &self,
        key: &Key,
        modifiers: Modifiers,
    ) -> Option<CommandId> {
        let binding_key = binding_key_from_iced(key)?;
        let binding = KeyBinding {
            key: binding_key,
            modifiers: modifiers.bits(),
        };
        self.key_map.get(&binding).cloned()
    }
}

/// Platform config path for the optional user bindings file.
pub fn user_bindings_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "Silent Orb", "aura-editor")
        .map(|dirs| dirs.config_dir().join("bindings.json"))
}

fn parse_key_binding(
    key: &str,
    modifiers: &[String],
) -> Result<KeyBinding, String> {
    let modifiers = parse_modifiers(modifiers)?;
    let binding_key = parse_binding_key(key)?;
    Ok(KeyBinding {
        key: binding_key,
        modifiers: modifiers.bits(),
    })
}

fn parse_modifiers(names: &[String]) -> Result<Modifiers, String> {
    let mut modifiers = Modifiers::empty();
    for name in names {
        match name.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CTRL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "logo" | "super" | "meta" | "cmd" | "command" => {
                modifiers |= Modifiers::LOGO
            }
            other => {
                return Err(format!("unknown modifier `{other}`"));
            }
        }
    }
    Ok(modifiers)
}

fn parse_binding_key(key: &str) -> Result<BindingKey, String> {
    let normalized = key.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(String::from("empty key"));
    }

    if let Some(named) = named_key_from_str(&normalized) {
        return Ok(BindingKey::Named(named));
    }

    // Single-character (or short character) keys.
    if normalized.chars().count() == 1 {
        return Ok(BindingKey::Character(normalized));
    }

    Err(format!("unsupported key `{key}`"))
}

fn binding_key_from_iced(key: &Key) -> Option<BindingKey> {
    match key.as_ref() {
        Key::Character(text) => {
            let normalized = text.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                None
            } else {
                Some(BindingKey::Character(normalized))
            }
        }
        Key::Named(named) => Some(BindingKey::Named(named)),
        Key::Unidentified => None,
    }
}

fn named_key_from_str(name: &str) -> Option<Named> {
    Some(match name {
        "escape" | "esc" => Named::Escape,
        "enter" | "return" => Named::Enter,
        "tab" => Named::Tab,
        "space" | "spacebar" => Named::Space,
        "backspace" => Named::Backspace,
        "delete" | "del" => Named::Delete,
        "arrowup" | "up" => Named::ArrowUp,
        "arrowdown" | "down" => Named::ArrowDown,
        "arrowleft" | "left" => Named::ArrowLeft,
        "arrowright" | "right" => Named::ArrowRight,
        "home" => Named::Home,
        "end" => Named::End,
        "pageup" | "page_up" => Named::PageUp,
        "pagedown" | "page_down" => Named::PageDown,
        "f1" => Named::F1,
        "f2" => Named::F2,
        "f3" => Named::F3,
        "f4" => Named::F4,
        "f5" => Named::F5,
        "f6" => Named::F6,
        "f7" => Named::F7,
        "f8" => Named::F8,
        "f9" => Named::F9,
        "f10" => Named::F10,
        "f11" => Named::F11,
        "f12" => Named::F12,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_interfaces::{CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_OPEN};

    fn known() -> HashSet<String> {
        HashSet::from([
            CMD_DOCUMENT_OPEN.to_string(),
            CMD_DOCUMENT_CLOSE_ACTIVE.to_string(),
        ])
    }

    #[test]
    fn parses_default_bindings() {
        let bindings = Bindings::from_json_str(DEFAULT_BINDINGS_JSON, &known()).unwrap();
        assert_eq!(bindings.len(), 2);

        let open = bindings.resolve(
            &Key::Character("o".into()),
            Modifiers::CTRL,
        );
        assert_eq!(open.as_ref().map(CommandId::as_str), Some(CMD_DOCUMENT_OPEN));

        let close = bindings.resolve(
            &Key::Character("W".into()),
            Modifiers::CTRL,
        );
        assert_eq!(
            close.as_ref().map(CommandId::as_str),
            Some(CMD_DOCUMENT_CLOSE_ACTIVE)
        );
    }

    #[test]
    fn rejects_unsupported_version() {
        let err = Bindings::from_json_str(
            r#"{"version": 99, "bindings": []}"#,
            &known(),
        )
        .unwrap_err();
        assert!(err.contains("unsupported bindings version"));
    }

    #[test]
    fn skips_unknown_command() {
        let bindings = Bindings::from_json_str(
            r#"{
              "version": 1,
              "bindings": [
                {
                  "command": "future.unknown",
                  "when": { "type": "key", "key": "x", "modifiers": [] }
                },
                {
                  "command": "document.open",
                  "when": { "type": "key", "key": "o", "modifiers": ["ctrl"] }
                }
              ]
            }"#,
            &known(),
        )
        .unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(
            bindings
                .resolve(&Key::Character("o".into()), Modifiers::CTRL)
                .as_ref()
                .map(CommandId::as_str),
            Some(CMD_DOCUMENT_OPEN)
        );
    }

    #[test]
    fn modifiers_must_match_exactly() {
        let bindings = Bindings::from_json_str(DEFAULT_BINDINGS_JSON, &known()).unwrap();
        assert!(bindings
            .resolve(
                &Key::Character("o".into()),
                Modifiers::CTRL | Modifiers::SHIFT
            )
            .is_none());
        assert!(bindings
            .resolve(&Key::Character("o".into()), Modifiers::empty())
            .is_none());
    }

    #[test]
    fn load_chain_prefers_user_file() {
        let dir = std::env::temp_dir().join(format!(
            "aura-editor-bindings-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let user = dir.join("bindings.json");
        fs::write(
            &user,
            r#"{
              "version": 1,
              "bindings": [
                {
                  "command": "document.close_active",
                  "when": { "type": "key", "key": "q", "modifiers": ["ctrl"] }
                }
              ]
            }"#,
        )
        .unwrap();

        let bindings =
            Bindings::load_chain(Some(&user), DEFAULT_BINDINGS_JSON, &known());
        assert_eq!(bindings.len(), 1);
        assert_eq!(
            bindings
                .resolve(&Key::Character("q".into()), Modifiers::CTRL)
                .as_ref()
                .map(CommandId::as_str),
            Some(CMD_DOCUMENT_CLOSE_ACTIVE)
        );
        assert!(bindings
            .resolve(&Key::Character("o".into()), Modifiers::CTRL)
            .is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_chain_falls_back_to_default_when_user_missing() {
        let missing = PathBuf::from("/tmp/aura-editor-bindings-does-not-exist.json");
        let bindings =
            Bindings::load_chain(Some(&missing), DEFAULT_BINDINGS_JSON, &known());
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn load_chain_empty_when_default_invalid() {
        let bindings = Bindings::load_chain(None, "{not json", &known());
        assert!(bindings.is_empty());
    }

    #[test]
    fn named_key_binding() {
        let bindings = Bindings::from_json_str(
            r#"{
              "version": 1,
              "bindings": [
                {
                  "command": "document.open",
                  "when": { "type": "key", "key": "escape", "modifiers": [] }
                }
              ]
            }"#,
            &known(),
        )
        .unwrap();
        assert_eq!(
            bindings
                .resolve(&Key::Named(Named::Escape), Modifiers::empty())
                .as_ref()
                .map(CommandId::as_str),
            Some(CMD_DOCUMENT_OPEN)
        );
    }
}
