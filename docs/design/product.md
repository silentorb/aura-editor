# Product design

Declarative product requirements for Aura Editor. Motivation and goals: [`../scoping/product.md`](../scoping/product.md).

## Role

- Aura Editor is the GUI frontend and DAW-like surface for Aura.
- It is a music/audio **designer**: workflows emphasize procedural and declarative design over traditional DAW-style manual editing of music and audio data.
- Users can hand-craft data and configuration; that capability must not displace Aura’s procedural, declarative model as the product’s center.

## Repository boundaries

- **aura-editor** (this repo) owns the editor UI product, its docs, and (when added) its application stack.
- **aura-engine** remains an independent, headless product; using or building the engine must not require this editor.
- **aura-workbench** owns Dev Container / Compose orchestration and multi-repo mounts.

Repos and projects use kebab-case.

## Dependencies

- The editor may share core dependencies with aura-engine.
- GUI and OS-integration dependencies belong in this repo (or editor-owned packages), not in aura-engine.

## Non-goals / avoided approaches

- Making aura-engine depend on editor UI or OS-integration stacks
- Positioning the product primarily as a traditional DAW for manually editing music/audio data
- Treating hand-crafted data as a replacement for procedural, declarative composition rather than a complement to it
- Embedding workbench orchestration (Compose/Dev Container entrypoint) inside this editor repo

Background: [`../scoping/background.md`](../scoping/background.md).
