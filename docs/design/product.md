# Product design

Declarative product requirements for Aura Editor. Motivation and goals: [`../scoping/product.md`](../scoping/product.md).

## Role

- Aura Editor is the GUI frontend and DAW-like surface for Aura.
- It is a music/audio **designer**: workflows emphasize procedural and declarative design over traditional DAW-style manual editing of music and audio data.
- Users can hand-craft data and configuration; that capability must not displace Aura’s procedural, declarative model as the product’s center.
- **Imp graphs** are the primary persistent resources users open and work with (not traditional DAW project bundles).

## Application stack

- The editor is a Rust desktop application.
- The primary GUI framework is **Iced**.
- Application logic is structured so unit and headless functional tests can exercise behavior without a real display (see [`testing.md`](testing.md)).

## Documents and MDI

- The editor is a multi-document interface: multiple graph documents may be open in separate tabs.
- Users can open a single-graph Aura JSON Imp file (`type: "imp-graph"` plus `nodes` / `edges`), switch tabs, and close documents.
- Opening a path that is already open activates that tab (no duplicate).
- An open document stores its document-type chain and tip payload (Imp `Graph` for `imp-graph`).
- The document content pane resolves a **view** by walking the type chain from tip to root and using the first role with an enabled view (`imp-graph` → builtin `graph-dag` today).
- Open is read-only for now: creating, modifying, and saving documents are out of scope until designed.
- Extensibility (commands, views, and document types) follows [`extensions.md`](extensions.md).

## Graph files

- Aura JSON documents declare a root string **`type`** naming the document-type id.
- Supported open format today: Aura JSON with `type: "imp-graph"` and a single Imp graph object (`nodes` + `edges`).
- Imp **GraphLibrary** Aura JSON (`type: "imp-graph-library"`, plus `id` + `definitions`) is recognized then rejected with a distinct, typed error (no session tab) until catalog open is designed.
- File open uses a native dialog; tests and tooling may open by path without a dialog.

## Graph visualization

- Open graphs are drawn on an Iced Canvas using automatic layout only (petgraph-backed layered layout).
- Users may pan and zoom the canvas; they must not freely drag nodes to persist positions.
- Editing nodes/connections later regenerates layout.
- Port-level connection detail is not required in the initial view (node-level edges are enough).

## Input bindings

- User input maps to editor **commands** through a configurable bindings layer (keyboard first). See [`input-bindings.md`](input-bindings.md).
- Commands are extension components registered by stable string id; see [`extensions.md`](extensions.md).
- Bindings load from an optional user JSON file, else a bundled default, else empty; the app runs without bindings.
- Toolbar and other UI may invoke the same commands without going through bindings.

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
- Persisting pan/zoom camera per document across sessions
- Opening GraphLibrary catalogs as editor documents until that flow is designed
- Creating, editing, or saving graph documents in the current read-only open slice
- Overriding native OS input commands via the bindings layer

Background: [`../scoping/background.md`](../scoping/background.md).
