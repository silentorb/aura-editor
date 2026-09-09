# Aura Editor — Agent Instructions

## Project

Aura Editor (`aura-editor`) is the GUI frontend and DAW-like surface for Aura. It is oriented as a music/audio **designer** rather than a traditional DAW for manually editing music/audio data. The application stack is Rust with **Iced**.

This repository stays separate so **aura-engine** can remain an independent, headless product. The editor will likely share most engine dependencies and add a larger GUI and OS-integration stack the engine does not need.

It provides means to hand-craft data and configuration while preserving Aura’s procedural, declarative spirit.

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

This file governs **aura-editor** only. Orchestration (Compose, Dev Container, multi-repo mounts) lives in **aura-workbench**, not here.

## Commands

From the repo root (with GUI system deps available):

```bash
cargo build
cargo test
cargo clippy
cargo run
```

## Naming

Repos and projects use **kebab-case** (`aura-editor`). Rust crates use **snake_case** (`aura_editor`).

## Working conventions

- Follow [`docs/rust-style.md`](docs/rust-style.md) for shared Aura/Imp-aligned Rust conventions, including error handling (DRY link to aura-engine’s style guide).

## Where to look

- [`docs/scoping/`](docs/scoping/) — product motivation, goals, background
- [`docs/design/`](docs/design/) — declarative product design (including [`testing.md`](docs/design/testing.md))
- [`docs/rust-style.md`](docs/rust-style.md) — Rust conventions and error handling (links to aura-engine)
