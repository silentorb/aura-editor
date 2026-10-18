# Extension system scoping

Motivation for Aura Editor’s extension model — libraries of components wired by configuration, not global plugins.

## Why not plugins

Plugins typically modify global host behavior through unconstrained hooks. The host framework is general-purpose, so it cannot effectively arbitrate conflicts between specific plugins. Two plugins can fight over the same surface, and composition is accidental.

## Components + configuration

A better model (inspired by [Tome’s extension system](https://github.com/silentorb/tome/blob/main/docs/features/extensions.md)):

- **Extensions** are packages that publish **components** (specific capabilities).
- Extensions themselves carry little configuration.
- **Project/host configuration** integrates component instances into the rest of the product (roles, enablement, params).
- The host arbitrates integration via config (which component fills which role), not via racey global hooks.

## First component kinds

- **`command`** — bindable editor actions identified by stable string ids (toolbar, keyboard bindings, tests).
- **`view`** — MDI content presenters; a registry maps document roles to views that can show them.
- **`document-type`** — intended later: match/open/load for resource kinds; open flow becomes detect type → session document → default view by role.

## Rust loading (v1)

Tome loads extension modules with Bun dynamic `import()`. Rust has no equivalent without ABI plugins. v1 uses **compile-time linked** extension crates plus `extensions.json` for enablement and role binding: extension packages depend on `aura_interfaces` only; the editor binary links them and calls each `register` at startup. Dynamic `.so`/`.dll` loading is deferred unless compile-time linking becomes insufficient.

## Boundaries

The extension system for views, commands, and (later) document types lives in **aura-editor**. **aura-engine** remains headless Imp render; engine-facing kinds, if ever needed, must not pull GUI into the engine.

Refined requirements: [`../design/extensions.md`](../design/extensions.md).
