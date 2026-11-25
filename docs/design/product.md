# Product design

Declarative product requirements for Aura Editor. Motivation and goals: [`../scoping/product.md`](../scoping/product.md).

## Role

- Aura Editor is Aura's **editor UI**: everything audio- or music-specific a user sees while designing with Aura.
- It is a music/audio **designer**: workflows emphasize procedural and declarative design over traditional DAW-style manual editing of music and audio data. Hand-crafted data (notation) complements procedural Imp graphs; it does not displace them.
- **Imp graphs** are the primary persistent resources users work with, edited in Imp Lab's built-in graph editor. Aura Editor adds editors for Aura's own record kinds, starting with **notation** ([`piano-roll.md`](piano-roll.md)).

## Layering

- Aura Editor runs **inside Imp Lab** as extensions, not as its own application. Imp Lab ([imp-lab](https://github.com/silentorb/imp-lab)) is a domain-neutral host for Imp graphs and Tome records; Aura Editor supplies the Aura-specific UI.
- **All audio-specific Tauri UI lives in this repo.** Imp Lab must not contain audio or music concepts (notation, piano roll, audition, recording), even behind generic names; when a piece of Aura UI needs something Lab doesn't offer, the answer is additional, domain-neutral extension support in Imp Lab, not Aura code in Lab.
- Aura Editor provides **modular components**: its UI library exports implementations by id, and the Aura pack's `extensions.json` chooses and configures them. Nothing here wires itself in.
- There are no document types: Aura records are **Tome node types**. Notation records are members of the Notations type table `01M80M9N0TAT00000000000001`, which is also the Imp node type of a notation record (`aura_imp::NOTATION_NODE_TYPE`); the two ids stay equal.
- Aura engine functionality (catalogs, playback, MIDI capture, plugin hosting) comes from **aura-engine**'s extension libraries. Editors talk to them only through the services and topics the pack wires up.

## Distribution

- What ships is `imp-lab.exe` plus the **Aura pack** beside it: `extensions.json` (owned here, [`pack.md`](pack.md)), aura-engine's extension libraries and data, and this repo's UI library.
- Without the pack, Imp Lab runs as plain Imp Lab; Aura records are listed but not openable.

## Repository boundaries

- **aura-editor** (this repo) owns Aura's editor UI implementations, the Aura pack configuration, and their docs.
- **imp-lab** owns the host: projects, records, the graph editor, the extension system and its UI contract (`imp-lab-ui`).
- **aura-engine** remains an independent, headless product with the pack's extension libraries (`aura_imp_lab_catalogs`, `aura_imp_lab_playback`) and data (`lab-pack/catalogs`, `lab-pack/demo-project`); using or building the engine must not require this repo.
- **aura-workbench** owns Dev Container / Compose orchestration, multi-repo mounts and pack assembly / export scripts.

Repos and projects use kebab-case.

## Stack

- TypeScript + React, built to one ES module (`dist/aura_editor.js`) with Vite in library mode.
- React and Mantine are Imp Lab's shared instances (`imp-lab-ui` `SHARED_MODULE_IDS`), never bundled; CSS is inlined into the module.
- Tests run with `bun test`.

## Non-goals / avoided approaches

- A separate Aura desktop application or GUI framework (the Iced editor was removed; Imp Lab is the host)
- Audio-specific code in Imp Lab, or Imp Lab depending on Aura crates or packages
- Document types layered over Tome node types
- Self-registering UI code (implementations are wired by configuration)
- Making aura-engine depend on editor UI
- Positioning the product primarily as a traditional DAW for manually editing music/audio data
