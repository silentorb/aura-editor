//! Playback helpers for Imp graphs and notation audition.

use std::collections::BTreeMap;
use std::path::Path;

use aura_ext_notation::Notation;
use aura_imp::{
    aura_registry, graph_from_json_str, discover_graph_libraries, aura_registry_with_libraries,
    GraphJsonError, NodeId, Registry, RegistryError,
};
use aura_integration::{
    translate_graph_to_sampler_with_bindings, translate_graph_to_sampler_with_options,
    TranslateOptions,
};
use aura_render::Sampler;
use aura_sample::SampleRate;
use aura_sequencing::{
    beats_to_seconds, NoteEvent, Score, ScoreSignal, Semitone, Tempo, TimeSignature,
};
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
    #[error("invalid notation tempo: {0}")]
    InvalidTempo(String),
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
/// not share a single `imp_core_types` package identity. `neighbor_hint` is used
/// for sibling library discovery and as the graph directory for `notation_score` paths.
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
    let graph_dir = neighbor_hint.and_then(Path::parent);
    let empty_bindings = BTreeMap::new();
    let sampler = translate_graph_to_sampler_with_options(
        &engine_graph,
        &registry,
        sample_rate,
        TranslateOptions {
            score_bindings: &empty_bindings,
            graph_dir,
        },
    )
    .map_err(|error| PlaybackPrepareError::Translate(error.to_string()))?;
    Ok((node_id, sampler))
}

/// Maps editor [`Notation`] onto engine [`Score`] (same ontology as generators).
pub fn notation_to_score(notation: &Notation) -> Result<Score, PlaybackPrepareError> {
    let tempo = Tempo::new(notation.tempo_bpm)
        .map_err(|error| PlaybackPrepareError::InvalidTempo(error.to_string()))?;
    let time_signature = TimeSignature {
        beats_per_bar: notation.time_signature.beats_per_bar,
        beat_unit: notation.time_signature.beat_unit,
    };
    let events = notation
        .events
        .iter()
        .map(|note| {
            NoteEvent::new(
                Semitone(note.semitone),
                note.start_beats,
                note.duration_beats,
            )
            .with_velocity(note.velocity)
        })
        .collect();
    Ok(Score::new(time_signature, tempo, events))
}

/// Converts parked playhead beats to absolute seconds at the notation tempo.
pub fn playhead_start_secs(notation: &Notation, playhead_beats: f64) -> Result<f64, PlaybackPrepareError> {
    let tempo = Tempo::new(notation.tempo_bpm)
        .map_err(|error| PlaybackPrepareError::InvalidTempo(error.to_string()))?;
    Ok(beats_to_seconds(playhead_beats.max(0.0), tempo))
}

/// Converts playback seconds back to beats at the notation tempo.
pub fn secs_to_beats(notation: &Notation, secs: f64) -> Result<f64, PlaybackPrepareError> {
    let tempo = Tempo::new(notation.tempo_bpm)
        .map_err(|error| PlaybackPrepareError::InvalidTempo(error.to_string()))?;
    let spb = tempo.seconds_per_beat();
    if spb <= 0.0 {
        return Ok(0.0);
    }
    Ok(secs / spb)
}

/// Builds a bound Imp audition graph: `external_score` → `poly_sine_score` → `out`.
///
/// The editor models an open notation document as a nullary `external_score` node with
/// Aura-embedded [`Notation`] data; callers pass that embedded payload here. On-disk
/// `type: "notation"` files are unchanged and are only projected at open/save.
pub fn prepare_notation_playback(
    notation: &Notation,
    sample_rate: SampleRate,
) -> Result<(Box<dyn Sampler>, f64), PlaybackPrepareError> {
    let score = notation_to_score(notation)?;
    let duration_secs = score.duration_secs().max(0.0);
    let signal = ScoreSignal::finite(score);

    let graph_json = r#"{
  "nodes": {
    "score": { "id": "score", "type": "external_score", "inputs": {} },
    "clock": { "id": "clock", "type": "time", "inputs": {} },
    "voice": { "id": "voice", "type": "poly_sine_score", "inputs": {} },
    "out": { "id": "out", "type": "output", "inputs": {} }
  },
  "edges": {
    "score_to_voice": {
      "from": { "node": "score", "port": "score" },
      "to": { "node": "voice", "port": "score" }
    },
    "time_to_voice": {
      "from": { "node": "clock", "port": "time" },
      "to": { "node": "voice", "port": "time" }
    },
    "voice_to_out": {
      "from": { "node": "voice", "port": "audio" },
      "to": { "node": "out", "port": "value" }
    }
  }
}"#;

    let engine_graph =
        graph_from_json_str(graph_json).map_err(PlaybackPrepareError::Serialize)?;
    let registry = aura_registry()?;
    let bindings: BTreeMap<NodeId, ScoreSignal> =
        BTreeMap::from([(String::from("score"), signal)]);
    let sampler =
        translate_graph_to_sampler_with_bindings(&engine_graph, &registry, sample_rate, &bindings)
            .map_err(|error| PlaybackPrepareError::Translate(error.to_string()))?;
    Ok((sampler, duration_secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_ext_notation::NotationNote;

    #[test]
    fn notation_to_score_maps_events() {
        let mut notation = Notation::empty();
        notation.events.push(NotationNote::new(60, 0.0, 0.5));
        notation.events.push(NotationNote::new(64, 0.0, 0.5));
        let score = notation_to_score(&notation).expect("score");
        assert_eq!(score.events.len(), 2);
        assert!((score.duration_beats() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn prepare_notation_playback_is_audible_and_pure() {
        let mut notation = Notation::empty();
        notation.events.push(NotationNote::new(60, 0.0, 0.5));
        notation.events.push(NotationNote::new(64, 0.0, 0.5));
        let (sampler, duration) =
            prepare_notation_playback(&notation, SampleRate::RATE_44100).expect("prepare");
        assert!(duration > 0.0);
        let sample = sampler.at(0.05);
        assert!(sample.abs() > 1e-4, "expected audible sample, got {sample}");
        assert_eq!(sampler.at(0.05), sampler.at(0.05));
    }
}
