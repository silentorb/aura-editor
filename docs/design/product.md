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
- Users can open a single-graph Imp JSON file (`{ nodes, edges }`), switch tabs, and close documents.
- Opening a path that is already open activates that tab (no duplicate).
- An open document holds the parsed Imp `Graph` in memory.
- The document content pane may be blank until graph visualization is implemented.
- Open is read-only for now: creating, modifying, and saving documents are out of scope until designed.

## Graph files

- Supported open format today: a single Imp graph object (`nodes` + `edges`).
- Imp **GraphLibrary** files (`id` + `definitions` with embedded `body` graphs) are not opened yet; attempting to open one must fail with a distinct, typed error.
- File open uses a native dialog; tests and tooling may open by path without a dialog.

## Deferred visualization

- Graph node visualization and automatic layout are not required yet.
- When visualization is added: layout must be automatic only; users must not freely drag nodes to persist positions. Editing nodes/connections later regenerates layout.

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
- Day-to-day development uses WSL Linux containers. End users target Windows. A dedicated Windows cross-compile container in workbench is a future requirement, not a current one.

## Non-goals / avoided approaches

- Making aura-engine depend on editor UI or OS-integration stacks
- Positioning the product primarily as a traditional DAW for manually editing music/audio data
- Treating hand-crafted data as a replacement for procedural, declarative composition rather than a complement to it
- Embedding workbench orchestration (Compose/Dev Container entrypoint) inside this editor repo
- Running the aura-engine test suite inside the editor container
- Shipping or requiring a Windows cross-compile toolchain in the current skeleton
- Graph canvas / node visualization in the current open-document slice
- Freeform manual node placement on a canvas (when visualization lands, layout stays automatic)
- Opening GraphLibrary catalogs as editor documents until that flow is designed
- Creating, editing, or saving graph documents in the current read-only open slice

Background: [`../scoping/background.md`](../scoping/background.md).
