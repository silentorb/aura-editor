# Product design

Declarative product requirements for Aura Editor. Motivation and goals: [`../scoping/product.md`](../scoping/product.md).

## Role

- Aura Editor is the GUI frontend and DAW-like surface for Aura.
- It is a music/audio **designer**: workflows emphasize procedural and declarative design over traditional DAW-style manual editing of music and audio data.
- Users can hand-craft data and configuration; that capability must not displace Aura’s procedural, declarative model as the product’s center.

## Application stack

- The editor is a Rust desktop application.
- The primary GUI framework is **Iced**.
- Application logic is structured so unit and headless functional tests can exercise behavior without a real display (see [`testing.md`](testing.md)).

## Repository boundaries

- **aura-editor** (this repo) owns the editor UI product, its docs, and its application stack.
- **aura-engine** remains an independent, headless product; using or building the engine must not require this editor.
- **aura-workbench** owns Dev Container / Compose orchestration and multi-repo mounts.

Repos and projects use kebab-case. Rust packages use snake_case (`aura_editor`).

## Dependencies

- The editor may share core dependencies with aura-engine.
- GUI and OS-integration dependencies belong in this repo (or editor-owned packages), not in aura-engine.

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

Background: [`../scoping/background.md`](../scoping/background.md).
