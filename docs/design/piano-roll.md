# Piano roll

Declarative requirements for the hand-crafted notation piano-roll view. Product context: [`product.md`](product.md). Extensibility: [`extensions.md`](extensions.md). Motivation: [`../scoping/product.md`](../scoping/product.md).

**Active host:** [Imp Lab](https://github.com/silentorb/imp-lab) with Aura Lab pack playback. The Iced implementation in this repo is historical reference only — do not extend it.

## Role

- Users can create and edit discrete note events on a time × pitch grid — a classic MIDI piano roll — without making destructive timeline editing the product’s center.
- Notation is a **complement** to procedural Imp graphs: one plain-text Aura JSON file per notation document, composed later with other files like a programming project (including Imp graphs that wire notation tips by tip NodeId through Miro host resolve).
- In the editor session, a notation tip is edited as Aura JSON (`type: "notation"`); on-disk tip files are unchanged and are only projected at open/save. (The deprecated Iced host wrapped the tip as a one-node session model; Imp Lab keeps the tip as the document body.)
- Embedded notation is the same musical material procedural generators produce: an engine `Score` / `ScoreSignal`. Play from a piano-roll tab auditions that score directly (poly-sine sampler via the Aura Lab playback extension), sharing the Score ontology with `arpeggio` / `bass_line` / `drum_grid` / tip-NodeId remotes.

## Document type

- Aura JSON tip id / role: `notation`.
- Open stack: `json` → `aura-json` → `notation`.
- On-disk shape (version 1):

```json
{
  "type": "notation",
  "id": "01JNOTATIONTIP000000000001",
  "title": "arp",
  "version": 1,
  "tempoBpm": 120,
  "timeSignature": { "beatsPerBar": 4, "beatUnit": 4 },
  "events": [
    { "semitone": 60, "startBeats": 0.0, "durationBeats": 0.25, "velocity": 1.0 }
  ]
}
```

- Pitch uses scientific notation labels with middle C = `C4` = semitone `60` (A4 = 69), matching Aura engine `Semitone` encoding.
- Event fields align with engine `NoteEvent` / `Score`. The editor keeps its own `aura_ext_notation` serde model for the piano roll; the engine loads the same on-disk schema when host-resolving NodeIds. Schemas must stay compatible. Document-level `id` is required for Miro host indexing. Optional `title` is the same display title Imp nodes carry; anything that maps to a node may have an optional title.
- Per-note `params` are omitted in v1; `velocity` defaults to `1.0` when absent.

## Create, open, and save

- Users may **New** an empty notation document (untitled tab), **Open** an existing `notation` file, and **Save** (Save As when untitled).
- Imp Lab open/save round-trips the Aura JSON tip directly. (Iced historically wrapped Open/New as a one-node `ModelGraph` and saved only the embedded tip.)
- Edits mark the document dirty; a successful save clears dirty.
- Untitled tabs are not restored via open-documents user state.
- Imp graph documents are edited in the graph view, not in the piano roll.

## View

- Content pane shows the piano-roll UI when the open document kind is `notation` (Imp Lab). Iced historically resolved role `notation` → view `piano-roll`.
- Layout: left vertical piano lane (chromatic keys with labels such as `C4`, `D#3`); right time × pitch grid with note rectangles.
- A fixed **ruler strip** sits at the top of the grid. Clicking the ruler parks the **playhead** (respecting snap). A vertical playhead line runs through the ruler and grid.
- While Play is active, the playhead display advances with the CPAL sample clock; on Stop (or end of score) it snaps back to the parked start position.
- Grid lines follow the active snap subdivision. Measure/bar awareness is deferred.
- Default visible pitch band is approximately `C2`–`C6`, with pan/zoom of the grid viewport.

## Playback

- **Start** / **Stop** / **Toggle** (Play menu and Space) from a notation tab audition the open document’s events via the Aura Lab playback extension (`Score` → poly-sine), starting at the parked playhead time.
- Playback auto-stops when the transport reaches the score duration; the playhead snaps back to its parked position.
- Imp graph tabs keep the existing graph → CPAL path (from `t = 0`).

## Modal tools

Interaction is **tool-modal**: left-click / drag meaning depends on the active tool (not a single context-crammed click model).

| Tool | Default binding | Cursor | Primary action |
|------|-----------------|--------|----------------|
| Draw | `D` | Pen overlay (tip on hotspot) | Click empty grid cell → place note at snapped start and pitch; duration equals the current snap size in beats |
| Select | `S` | Default pointer; horizontal resize on note edges | Click note to select; drag body to move (respecting snap when enabled); drag left/right edge to resize duration; click empty space clears selection |
| Erase | `E` | Eraser overlay (corner on hotspot) | Click note to delete |

Ruler clicks set the playhead regardless of the active tool.

Tool state is editor-global (not per document). Playhead parking is session-only per open notation document.

## Snap

- Snap-to-grid is toggleable; default binding `N`.
- Snap is **on** by default.
- Configurable resolution via toolbar dropdown: `1/4`, `1/8`, `1/16`, `1/32` of a beat (`0.25`, `0.125`, `0.0625`, `0.03125` beats).
- Default resolution: `1/16`.

## Extension and user state

- Imp Lab hosts notation as a first-class document kind (open/save/new) with piano-roll UI in the React shell; tool/snap prefs are session UI state (Iced historically persisted them via `aura-notation` / `piano-roll-ui`).
- Iced-only: linked package id `aura-notation` registered the document type, view, tool/save commands, and user-state entry (see [`user-state.md`](user-state.md) / [`extensions.md`](extensions.md)) — not extended further.

## Non-goals / avoided approaches

- Measure / bar chrome and timing-division UI beyond the snap subdivision
- Merging editor and engine notation serde into one shared crate in this slice (parallel compatible loaders are enough)
- Making the piano roll the primary authoring model over Imp graphs
- RMB-driven context menus for the three tools in v1
- Scrub-while-dragging the playhead during playback (park + start only in this slice)
