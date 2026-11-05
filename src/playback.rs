//! Playback helpers for resolving the session graph's audio sink.

use std::path::Path;

use aura_imp::{
    aura_registry_with_libraries, discover_graph_libraries, graph_from_json_str, GraphJsonError,
    Registry, RegistryError,
};
use aura_integration::translate_graph_to_sampler;
use aura_render::Sampler;
use aura_sample::SampleRate;
use imp_core_types::Graph;
use thiserror::Error;

/// Errors preparing in-memory session playback.
#[derive(Debug, Error)]
pub enum PlaybackPrepareError {
    #[error("graph has no output node")]
    MissingOutput,
    #[error("multiple output nodes are not supported")]
    MultipleOutputs,
    #[error("failed to serialize session graph: {0}")]
    Serialize(serde_json::Error),
    #[error("{0}")]
    GraphJson(#[from] GraphJsonError),
    #[error("{0}")]
    Registry(#[from] RegistryError),
    #[error("{0}")]
    Translate(String),
}

/// Resolves the most-output node on an Imp graph (prefer id `out`).
///
/// Mirrors `aura_integration::find_output_node` without sharing the engine's
/// `imp_core_types` crate instance (editor and engine mount Imp separately).
pub fn resolve_playback_node(graph: &Graph) -> Result<String, PlaybackPrepareError> {
    let outputs: Vec<_> = graph
        .nodes
        .values()
        .filter(|node| node.node_type == "output")
        .map(|node| node.id.clone())
        .collect();

    if outputs.iter().any(|id| id == "out") {
        return Ok(String::from("out"));
    }

    match outputs.as_slice() {
        [] => Err(PlaybackPrepareError::MissingOutput),
        [id] => Ok(id.clone()),
        _ => Err(PlaybackPrepareError::MultipleOutputs),
    }
}

/// Builds an Aura Imp registry for translating a session graph.
///
/// When `neighbor_hint` is set (typically the document path), discovers
/// neighboring graph libraries (e.g. `instruments.json`). Otherwise uses the
/// base Aura registry only. Persistence is not required for playback.
pub fn playback_registry(neighbor_hint: Option<&Path>) -> Result<Registry, PlaybackPrepareError> {
    let libraries = match neighbor_hint {
        Some(path) => discover_graph_libraries(path)?,
        None => Vec::new(),
    };
    Ok(aura_registry_with_libraries(&libraries)?)
}

/// Translates the in-memory session graph into a sampler for CPAL playback.
///
/// Round-trips JSON so the editor's Imp mount and the engine's Imp mount need
/// not share a single `imp_core_types` package identity. `neighbor_hint` is only
/// used for optional sibling library discovery.
pub fn prepare_session_playback(
    graph: &Graph,
    neighbor_hint: Option<&Path>,
    sample_rate: SampleRate,
) -> Result<(String, Box<dyn Sampler>), PlaybackPrepareError> {
    let node_id = resolve_playback_node(graph)?;
    let json = serde_json::to_string(graph).map_err(PlaybackPrepareError::Serialize)?;
    let engine_graph =
        graph_from_json_str(&json).map_err(PlaybackPrepareError::Serialize)?;
    let registry = playback_registry(neighbor_hint)?;
    let sampler = translate_graph_to_sampler(&engine_graph, &registry, sample_rate)
        .map_err(|error| PlaybackPrepareError::Translate(error.to_string()))?;
    Ok((node_id, sampler))
}
