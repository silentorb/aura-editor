# Piano roll

Declarative requirements for the hand-crafted notation piano-roll view. Product context: [`product.md`](product.md). Extensibility: [`extensions.md`](extensions.md). Motivation: [`../scoping/product.md`](../scoping/product.md).

## Role

- Users can create and edit discrete note events on a time × pitch grid — a classic MIDI piano roll — without making destructive timeline editing the product’s center.
- Notation is a **complement** to procedural Imp graphs: one plain-text Aura JSON file per notation document, composed later with other files like a programming project.
- This slice does not audition notation events: global Play streams the active Imp graph tab’s in-memory procedural audio. Start from a piano-roll tip fails clearly until notation→graph wiring exists.

## Document type

- Aura JSON tip id / role: `notation`.
- Open stack: `json` → `aura-json` → `notation`.
- On-disk shape (version 1):

```json
{
  "type": "notation",
  "version": 1,
  "tempoBpm": 120,
  "timeSignature": { "beatsPerBar": 4, "beatUnit": 4 },
  "events": [
    { "semitone": 60, "startBeats": 0.0, "durationBeats": 0.25, "velocity": 1.0 }
  ]
}
```

- Pitch uses scientific notation labels with middle C = `C4` = semitone `60` (A4 = 69), matching Aura engine `Semitone` encoding.
- Event fields align with engine `NoteEvent` / `Score` conceptually; the editor owns the serde schema in this slice (no engine serde dependency yet).
- Per-note `params` are omitted in v1; `velocity` defaults to `1.0` when absent.

## Create, open, and save

- Users may **New** an empty notation document (untitled tab), **Open** an existing `notation` file, and **Save** (Save As when untitled).
- Edits mark the document dirty; a successful save clears dirty.
- Untitled tabs are not restored via open-documents user state.
- Imp graph documents remain read-only in this slice.

## View

- Content pane uses the `piano-roll` view when the tip role is `notation`.
- Layout: left vertical piano lane (chromatic keys with labels such as `C4`, `D#3`); right time × pitch grid with note rectangles.
- Grid lines follow the active snap subdivision. Measure/bar awareness is deferred.
- Default visible pitch band is approximately `C2`–`C6`, with pan/zoom of the grid viewport.

## Modal tools

Interaction is **tool-modal**: left-click / drag meaning depends on the active tool (not a single context-crammed click model).

| Tool | Default binding | Cursor | Primary action |
|------|-----------------|--------|----------------|
| Draw | `D` | Pen overlay (tip on hotspot) | Click empty grid cell → place note at snapped start and pitch; duration equals the current snap size in beats |
| Select | `S` | Default pointer; horizontal resize on note edges | Click note to select; drag body to move (respecting snap when enabled); drag left/right edge to resize duration; click empty space clears selection |
| Erase | `E` | Eraser overlay (corner on hotspot) | Click note to delete |

Custom Draw/Erase cursors are canvas overlays (iced has no custom OS cursors). The pointer hotspot is fixed across tools; glyphs pivot around that hotspot so tool switches do not shift placement.

Tool state is editor-global (not per document).

## Snap

- Snap-to-grid is toggleable; default binding `N`.
- Snap is **on** by default.
- Configurable resolution via toolbar dropdown: `1/4`, `1/8`, `1/16`, `1/32` of a beat (`0.25`, `0.125`, `0.0625`, `0.03125` beats).
- Default resolution: `1/16`.

## Extension and user state

- Linked package id `aura-notation` registers the document type, view, tool/save commands, and a `piano-roll-ui` user-state entry in bucket `aura-notation`.
- Persisted UI prefs: active tool, snap enabled, snap resolution (see [`user-state.md`](user-state.md)).
- Host adapters own Iced widgets, command dispatch, and typed collect/apply for user state.

## Non-goals / avoided approaches

- Auditioning the open notation document’s events (no notation→graph wiring / playhead UI yet)
- Measure / bar chrome and timing-division UI beyond the snap subdivision
- Engine `aura_sequencing` serde or Imp literal-score nodes in this slice
- Making the piano roll the primary authoring model over Imp graphs
- RMB-driven context menus for the three tools in v1
