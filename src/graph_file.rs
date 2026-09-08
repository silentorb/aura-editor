//! Load single Imp graph files and typed open errors.

use std::fs;
use std::path::{Path, PathBuf};

use imp_core_types::Graph;
use serde_json::Value;
use thiserror::Error;

/// Failure while opening or parsing a graph document from disk.
///
/// Variants are distinct so callers can distinguish I/O, JSON, unsupported
/// GraphLibrary files, and other invalid shapes. Causes are preserved as text
/// so the type stays `Clone` for Iced messages.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpenError {
    /// Could not read the file.
    #[error("could not read graph file {}: {message}", path.display())]
    Io {
        /// Path that failed to open or read.
        path: PathBuf,
        /// Underlying I/O error message.
        message: String,
    },
    /// File contents were not valid JSON for a graph.
    #[error("invalid JSON in graph file {}: {message}", path.display())]
    Json {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying JSON error message.
        message: String,
    },
    /// File is an Imp GraphLibrary catalog (not yet supported).
    #[error(
        "graph library not supported yet: {} (open a single-graph {{ nodes, edges }} file)",
        path.display()
    )]
    GraphLibraryNotSupported {
        /// Path of the GraphLibrary file.
        path: PathBuf,
    },
    /// JSON parsed but is not a usable single Imp graph.
    #[error("invalid graph file {}: {reason}", path.display())]
    InvalidGraph {
        /// Path of the invalid file.
        path: PathBuf,
        /// Why the value is not a valid single graph.
        reason: String,
    },
}

impl OpenError {
    /// Short banner text suitable for the editor status line.
    pub fn status_message(&self) -> String {
        self.to_string()
    }
}

/// Loads a single Imp `{ nodes, edges }` graph from `path`.
///
/// GraphLibrary catalogs (`id` + `definitions`) return
/// [`OpenError::GraphLibraryNotSupported`].
pub fn load_graph_from_path(path: impl AsRef<Path>) -> Result<Graph, OpenError> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|err| OpenError::Io {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;
    load_graph_from_str(path.to_path_buf(), &text)
}

/// Parses a single Imp graph from JSON text, attributing errors to `path`.
pub fn load_graph_from_str(path: PathBuf, text: &str) -> Result<Graph, OpenError> {
    let value: Value = serde_json::from_str(text).map_err(|err| OpenError::Json {
        path: path.clone(),
        message: err.to_string(),
    })?;

    if looks_like_graph_library(&value) {
        return Err(OpenError::GraphLibraryNotSupported { path });
    }

    serde_json::from_value::<Graph>(value).map_err(|err| OpenError::InvalidGraph {
        path,
        reason: err.to_string(),
    })
}

fn looks_like_graph_library(value: &Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    obj.get("definitions")
        .is_some_and(|definitions| definitions.is_array())
        && obj.get("id").is_some_and(|id| id.is_string())
}
