# Piano roll

Declarative requirements for the hand-crafted notation editor. Product context: [`product.md`](product.md). Pack wiring: [`pack.md`](pack.md). Motivation: [`../scoping/product.md`](../scoping/product.md).

## Role

- Users can create and edit discrete note events on a time × pitch grid — a classic MIDI piano roll — without making destructive timeline editing the product’s center.
- Notation is a **complement** to procedural Imp graphs. Imp graphs reference notation records by node id; the engine resolves them from the project's data stores.
- Embedded notation is the same musical material procedural generators produce: an engine `Score` / `ScoreSignal`. Play from a notation tab auditions that score through the pack's playback service.
- The editor is the `notation-editor` implementation in this repo's UI library, wired to the Notations node type by the pack's `aura.notation-editor` component. Imp Lab hosts it and keeps the record's value, dirty state and undo history.

## Records

- A notation is a Tome record of node type **Notations** (`01M80M9N0TAT00000000000001`). Its value is the node-stored `notation` column (`json`), shape version 1:

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
- Event fields align with engine `NoteEvent` / `Score`; the editor's model (`src/notation/notation.ts`) and the engine load the same schema and must stay compatible. Per-note `params` are omitted in v1; `velocity` defaults to `1.0` when absent.
- The editor validates stored values when opening (`parse`); an invalid value fails to open with the reason.
- Users create notations with **File → New notation** or a store's **+** menu, and save them like any Imp Lab record (store picker on first save). Standalone notation JSON files (`"type": "notation"`) become records only through the seed tool ([`pack.md`](pack.md#sample-project)).

## View

- Layout: left vertical piano lane (chromatic keys with labels such as `C4`, `D#3`); right time × pitch grid with note rectangles.
- A fixed **ruler strip** sits at the top of the grid. Clicking the ruler parks the **playhead** (respecting snap). A vertical playhead line runs through the ruler and grid.
- While playing, the playhead follows the playback service's clock; on Stop (or end of score) it snaps back to the parked position.
- Grid lines follow the active snap subdivision. Measure/bar awareness is deferred.
- Default visible pitch band is approximately `C2`–`C6`, with pan/zoom of the grid viewport.
- The toolbar shows the tool, snap and audition controls, Record and Click when capture is offered, and Imp Lab's extension status items.

## Playback

- **Start** / **Stop** / **Toggle** (Play menu, Space) from a notation tab send the notation to the playback service with the parked playhead (`labStartBeats`), so it plays from there.
- Playback auto-stops when the transport reaches the score duration; the playhead snaps back to its parked position.
- Imp graph tabs play through Imp Lab's own command path (from `t = 0`).

## Audition

- While a note is placed, pressed or dragged, the editor sounds it through the configured service (`serviceId`) as service events: `{"type":"note_on","id","semitone","velocity"}`, retriggered on a pitch change, `{"type":"note_off","id"}` on pointer up, and `{"type":"all_notes_off"}` when the tab is left. The Aura pack voices them in streaming mode (`sine_note_voice`).
- Audition can be turned off (persisted) and goes quiet after the first failed send (no audio device, say).

## Recording

The editor records takes through the service's live note capture protocol (service requests). It doesn't know where the notes come from (the Aura pack records a MIDI input):

| Request | Reply |
| --- | --- |
| `{"type":"capture_info"}` | `{"topic"}`: the take topic. Any answer means capture is offered, and the toolbar shows **Record**. |
| `{"type":"record_start","notation":{…},"startBeats":n,"click":bool}` | `{"topic"}`. The service plays the notation from the playhead (with a click and a count-in when `click` is set) and starts the take. |
| `{"type":"record_stop"}` | `{"notes":[{semitone,startBeats,durationBeats,velocity}…]}`, the take as notation notes |

- While a take runs, the service publishes `{recording, countIn, notes}` on the take topic, with notes growing as keys are held; the piano roll draws them as an overlay.
- **Record** (`R`) starts a take from the parked playhead. The playhead follows the transport, and the editing tools are inert (panning still works). **Click** (metronome and one-bar count-in) is persisted.
- Stop (Record again, Space, or Play › Stop) adds the take to the notation as **one undoable edit** (overdub). If the topic turns `recording: false` (the transport was stopped some other way), the editor collects the take. Switching tabs or closing the notation ends the take first. An empty take adds nothing to history.

## Modal tools

Interaction is **tool-modal**: left-click / drag meaning depends on the active tool.

| Tool | Default binding | Cursor | Primary action |
|------|-----------------|--------|----------------|
| Draw | `D` | Pen overlay (tip on hotspot) | Click empty grid cell → place note at snapped start and pitch; duration equals the current snap size in beats |
| Select | `S` | Default pointer; horizontal resize on note edges | Click note to select; drag body to move (respecting snap when enabled); drag left/right edge to resize duration; click empty space clears selection |
| Erase | `E` | Eraser overlay (corner on hotspot) | Click note to delete |

Ruler clicks set the playhead regardless of the active tool.

## Snap

- Snap-to-grid is toggleable; default binding `N`. Snap is **on** by default.
- Resolution via toolbar dropdown: `1/4`, `1/8`, `1/16`, `1/32` of a beat. Default `1/16`.

## Commands and prefs

- The editor handles `notation.tool.draw` / `.select` / `.erase`, `notation.snap.toggle` and `notation.record.toggle`, and the pack's `play.*` commands while a notation tab is active (Imp Lab offers commands to the active editor first). Their default keys are the pack's `binding` components.
- Tool, snap, resolution, audition and click are editor-wide prefs (not per document), stored as the component's Imp Lab UI-state entry. Defaults: draw, snap on, `1/16`, audition on, click on. Playhead parking is per open tab and not persisted.

## Non-goals / avoided approaches

- Measure / bar chrome and timing-division UI beyond the snap subdivision
- Merging editor and engine notation serde into one shared package (parallel compatible loaders are enough)
- Making the piano roll the primary authoring model over Imp graphs
- RMB-driven context menus for the three tools in v1
- Scrub-while-dragging the playhead during playback (park + start only)
