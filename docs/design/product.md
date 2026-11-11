# Product design

Declarative product requirements for Aura Editor. Motivation and goals: [`../scoping/product.md`](../scoping/product.md).

## Role

- Aura Editor is the GUI frontend and DAW-like surface for Aura.
- It is a music/audio **designer**: workflows emphasize procedural and declarative design over traditional DAW-style manual editing of music and audio data.
- Users can hand-craft data and configuration; that capability must not displace Aura’s procedural, declarative model as the product’s center.
- **Imp graphs** are the primary persistent resources users open and work with (not traditional DAW project bundles).
- Hand-crafted **notation** (`notation` Aura JSON) is a second on-disk document kind: plain-text beat-grid note events stored as separate files, complementary to procedural graphs. See [`piano-roll.md`](piano-roll.md).
- **Documents are storage/UI envelopes.** On open, editable tips become an in-memory node graph (`ModelGraph`). An `imp-graph` file loads as its Imp nodes/edges; a `notation` file loads as a single nullary `external_score` node with Aura-embedded notation data (a complex primitive that outputs a static `score`). On-disk JSON shapes are unchanged; save projects the model back to the tip’s file format.

## Application stack

- The editor is a Rust desktop application.
- The primary GUI framework is **Iced**.
- Application logic is structured so unit and headless functional tests can exercise behavior without a real display (see [`testing.md`](testing.md)).

## Documents and MDI

- The editor is a multi-document interface: multiple documents may be open in separate tabs.
- Users can open a single-graph Aura JSON Imp file (`type: "imp-graph"` plus `nodes` / `edges`) or a notation document (`type: "notation"`), switch tabs, and close documents.
- Users may create a new empty notation document or a new empty Imp graph (untitled), edit it, and save it to disk (Save As when untitled).
- Opening a path that is already open activates that tab (no duplicate).
- An open document stores its document-type chain, view roles, and a node-backed tip (`ModelGraph` for `imp-graph` and `notation`; generic JSON remains an exception).
- The document content pane resolves a **view** by walking the type chain from tip to root and using the first role with an enabled view (`imp-graph` → builtin `graph-dag`; `notation` → `piano-roll`).
- Imp graph documents support create, modify, and save in the `graph-dag` view (see [`graph-dag.md`](graph-dag.md)).
- Notation documents support create, modify, and save in the piano-roll view, which edits the nullary node’s embedded notation (see [`piano-roll.md`](piano-roll.md)).
- Open document paths and the active tab are restored from ephemeral user state under the Aura user directory (see [`user-state.md`](user-state.md)); untitled tabs and pan/zoom are not.
- Shell body layout (documents and properties panes) is an Iced `pane_grid`: users may drag panes to rearrange and resize splits; the layout tree is persisted as user state (see [`user-state.md`](user-state.md)). The default arrangement is documents | properties, with Properties on the right of Documents. The active view’s tool strip lives inside the Documents pane (below the tab bar). A fixed status bar at the bottom of the window shows errors and transport feedback.
- Extensibility (commands, views, document types, and user-state) follows [`extensions.md`](extensions.md).

## Graph files

- Aura JSON documents declare a root string **`type`** naming the document-type id.
- Supported Imp open format: Aura JSON with `type: "imp-graph"` and a single Imp graph object (`nodes` + `edges`).
- Imp **GraphLibrary** Aura JSON (`type: "imp-graph-library"`, plus `id` + `definitions`) is recognized then rejected with a distinct, typed error (no session tab) until catalog open is designed.
- File open uses a native dialog; tests and tooling may open by path without a dialog.

## Notation score files

- Supported notation format: Aura JSON with `type: "notation"` and versioned tempo, time signature, and note events (see [`piano-roll.md`](piano-roll.md)).
- New File creates an empty notation tip in memory (nullary `external_score` node + empty embedded notation); Save writes `type: "notation"` Aura JSON from the embedded payload; Open wraps an existing notation file the same way and shows the piano-roll view.

## Graph visualization and editing

- Open graphs are drawn on an Iced Canvas using automatic layout only (petgraph-backed layered layout). See [`graph-dag.md`](graph-dag.md).
- Users may pan and zoom the canvas; they must not freely drag nodes to persist positions.
- Editing nodes or connections regenerates layout.
- Modal tools: Select / Add / Connect / Erase on the graph toolbar.
- Connection UI is toggleable globally: **Simple** (node→node with auto port pairing) or **Precise** (port handles, drag output→input).
- Graph view options (including connection mode) live in a far-right toolbar dropdown and apply across all graph tabs.

## Shell chrome

- The shell has a global **menu bar** with File / Edit / View / Play roots. The menu bar is fixed chrome above the dockable pane grid (not itself a rearrangeable pane).
- File menu exposes document commands (Open, New notation, New graph, Save, Close). Edit and View are reserved for future global actions.
- Play menu exposes transport commands (Start, Stop). Toggle playback is available as a command (default Space) and need not appear in the menu.
- Below the menu, the shell body is two named panes in an Iced `pane_grid`: **documents** and **properties**. Panes have title bars for drag rearrange; they are not closable or hideable in this slice. Split ratios are user-resizable.
- The **documents** pane hosts the tab bar, the active view’s tool strip (graph or piano-roll when applicable, otherwise empty), and the document content. Menus in this slice are not contextual to the active view.
- The **properties** pane shows graph node Id/Type editors when a graph node is selected; otherwise an empty-state message. Full input-literal inspection remains out of scope (see [`graph-dag.md`](graph-dag.md)).
- A fixed **status bar** below the pane grid shows the global status line (errors, graph-edit notices, playback state). It is not a rearrangeable pane and is not part of the persisted shell-layout tree.
- A global application toolbar, further tool panes, floating windows, closable docks, and per-view contextual menus are deferred.

## Playback

- **Start** / **Stop** / **Toggle** (`play.start`, `play.stop`, `play.toggle`) stream audio through CPAL (`PlaybackEngine`).
- **Imp graph tab:** translate the in-memory session graph (`translate` → sampler), starting at `t = 0`. The playable sink is that graph’s most-output node (prefer id `out`, else the sole `output` node). Saving is not required to play. Neighboring graph libraries may be discovered from the document path as a hint.
- **Notation / piano-roll tab:** map the open document to an engine `Score` (same type procedural generators emit), bind it through Imp `external_score` → `poly_sine_score`, and start at the parked playhead time (`start_at`). A ruler on the piano roll parks the playhead; while playing the line advances and snaps back on stop or end-of-score.
- Missing audio devices (common under WSL) surface a status error; the app must not panic.

## Input bindings

- User input maps to editor **commands** through a configurable bindings layer (keyboard first). See [`input-bindings.md`](input-bindings.md).
- Commands are extension components registered by stable string id; see [`extensions.md`](extensions.md).
- Bindings load from an optional user JSON file, else a bundled default, else empty; the app runs without bindings.
- Menus, toolbars, and other UI may invoke the same commands without going through bindings.

## Repository boundaries

- **aura-editor** (this repo) owns the editor UI product, its docs, and its application stack.
- **aura-engine** remains an independent, headless product; using or building the engine must not require this editor.
- **aura-workbench** owns Dev Container / Compose orchestration and multi-repo mounts.

Repos and projects use kebab-case. Rust packages use snake_case (`aura_editor`).

## Dependencies

- The editor may share core dependencies with aura-engine.
- GUI and OS-integration dependencies belong in this repo (or editor-owned packages), not in aura-engine.
- Shared Rust conventions (including error handling) follow [`../rust-style.md`](../rust-style.md).

## Dev containers (workbench)

- The workbench **editor** image may derive from the **engine** image while the engine image stays lean (shared Rust toolchain and light audio build deps).
- If the engine image later accumulates substantial dependencies the editor does not need, stop deriving editor from engine and introduce a shared base image instead.
- The editor container is for editor build, test, and run only. It is not a place to run the engine test suite.
- Day-to-day development uses WSL Linux containers. End users target Windows. Workbench provides an on-demand **editor-windows** service (`cargo-xwin`, `x86_64-pc-windows-msvc`) that writes `aura-editor.exe` to a configurable host output mount.

## Non-goals / avoided approaches

- Making aura-engine depend on editor UI or OS-integration stacks
- Positioning the product primarily as a traditional DAW for manually editing music/audio data
- Treating hand-crafted data as a replacement for procedural, declarative composition rather than a complement to it
- Embedding workbench orchestration (Compose / Dev Container entrypoint) inside this editor repo
- Running the aura-engine test suite inside the editor container
- Freeform manual node placement on a canvas (layout stays automatic)
- Persisting pan/zoom camera per document across launches
- Opening GraphLibrary catalogs as editor documents until that flow is designed
- Full input-literal property inspector for graph nodes (Properties pane: rename + change type only in this slice)
- Continuous scrub of the playhead while audio is running (park + start only)
- Explicit playback-node override / pinned target (Select playback node deferred; fringe cases TBD)
- Fully abstracting persistence from Aura operation (paths remain optional library-discovery hints)
- Overriding native OS input commands via the bindings layer

Background: [`../scoping/background.md`](../scoping/background.md).
