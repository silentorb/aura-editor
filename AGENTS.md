# Aura Editor — Agent Instructions

## Project

Aura Editor (`aura-editor`) is **Aura's editor UI**, delivered as **Imp Lab extensions**. It is not a separate application. **All audio-specific Tauri UI belongs in this repo** — never in [imp-lab](https://github.com/silentorb/imp-lab), which stays domain-neutral. When Aura UI needs something Lab can't do, add domain-neutral extension support to imp-lab and keep the Aura part here.

- `src/` is a UI library: it exports implementations by id (`src/index.ts`) against the `imp-lab-ui` contract and contains no wiring. React and Mantine come from Imp Lab (shared modules), never bundled.
- `pack/extensions.json` is the Aura pack configuration (owned here): it wires aura-engine's extension libraries and this UI library through components.
- There are no document types: Aura records are Tome node types (the notation editor edits Notations records).

## Source of truth

[`docs/README.md`](docs/README.md) is the entry point for all project documentation. **Design** docs under `docs/` define what the software must be (declarative requirements); code must conform to them. **Scoping** holds motivation and context. When docs and code disagree, update code or explicitly revise the doc — never silently drift.

## Scoping vs design

- **Scoping** (`docs/scoping/`) — gather motivation, goals, background, inspiration, context (write-friendly).
- **Design** (`docs/design/`) — refine into declarative requirements that define the product (read-friendly). Design may link to scoping for motivation origins.

Prefer gather then refine. When phases overlap, switch modes consciously. Do not keep an append-only decision log; edit living design in place.

## Capturing durable intent

Treat user instructions that define product behavior, architecture, or conventions as durable. Before or alongside code changes:

1. Capture **why** (goals, intent, background) in the appropriate **scoping** doc.
2. Capture **what must be true** (requirements, non-goals, avoided approaches) in the appropriate **design** doc.
3. Leave one-off task instructions as execution context only.

Plans that change durable intent must include an explicit scoping and/or design update step.

When reversing a direction: add a brief avoided-approach requirement in design; optionally note background in scoping; remove obsolete design prescriptions.

## Workspace

This file governs **aura-editor** only. Orchestration (Compose, Dev Container, multi-repo mounts, pack assembly and Windows export) lives in **aura-workbench**, not here. The imp-lab dependency is the `imp-lab-ui` package (sibling checkout `../imp-lab/packages/imp-lab-ui`, a Bun workspace).

## Commands

From the repo root (Bun; typically the workbench **imp-lab** service):

```bash
bun install
bun run test
bun run build
```

`bun run build` typechecks and writes `dist/aura_editor.js`. `bun run test` includes `pack/extensions.test.ts`, which checks the pack configuration against the engine's and the editor's ids; update it with the configuration.

## Naming

Repos and projects use **kebab-case** (`aura-editor`). Component ids in the pack use dotted `aura.*` ids; implementation ids are kebab-case (`notation-editor`).
