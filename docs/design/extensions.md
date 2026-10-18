# Extension system

Declarative requirements for Aura Editor extensions. Motivation: [`../scoping/extensions.md`](../scoping/extensions.md). Inspired by [Tome extensions](https://github.com/silentorb/tome/blob/main/docs/features/extensions.md).

## Role

- Most base editor capability is provided as **extension components**.
- Extensions are libraries of components. Integration is driven by **configuration**, not unconstrained global hooks.
- Integration **contracts** live in `aura_interfaces`. The **extension system** (this feature) handles discovery, config, and registration into host registries.
- Extension crates **must not** depend on the `aura_editor` application crate. They depend on `aura_interfaces` (and domain crates such as `imp_core_types` as needed).

## Loading (v1)

- Extensions are **compile-time linked** crates. The host lists linked packages and calls each package’s `register` entrypoint at startup.
- Registration alone does not activate a component: an enabled row in `extensions.json` must reference the component’s `implementationId` and `extensionId`.
- Dynamic library loading (`.so` / `.dll`) is out of scope for v1.

## Configuration

- Extension registration lives in an `extensions.json` document (bundled default for v1; project/user override paths may be added later).
- Top-level fields: `version` (number), `extensions` (array), `components` (array).
- Each extension has `id`, `enabled`, and optional `params`.
- Each component has `id`, `extensionId`, `kind`, `implementationId`, `enabled`, optional `roles`, optional `label`, and optional `params`.
- Hosts merge extension-level and component-level `params` (component wins on key conflict).
- Unsupported schema versions are a load failure for that file; the host must still start with empty registries when config is missing or invalid (with logging).
- No config UI and no marketplace in v1.

## Component kinds

### `command`

- Commands are identified by stable string ids.
- An extension registers a command implementation; config enables the component so the id appears in the host command registry.
- Input bindings and UI chrome invoke the same string ids. Unknown ids in a bindings file are skipped when they are not in the command registry.
- Builtin commands include `document.open` and `document.close_active`.

### `view`

- A view component declares `roles` (document roles it can present). When omitted, hosts may default `roles` to `[component.id]`.
- The host content pane resolves a document’s view through the view registry by role.
- v1 open path still loads Imp single-graph files in the host; those documents use the role `imp-graph`. The builtin `graph-dag` view claims that role.
- Iced `Element` construction may live in a host adapter so `aura_interfaces` does not depend on application `Message` types. View packages may still depend on Iced for their own widgets.
- When no enabled view implements a required role, the host shows an explicit error placeholder (not a panic).

### `document-type` (deferred)

- Future kind: match/open/load for a resource kind.
- Intended open pipeline: path/bytes → document-type → session document → default view by role (config arbitrates when multiple views claim a role).
- Not implemented in the v1 runtime; Imp-only open remains host hardwired until this kind lands.

## Builtin extension

- Package id `aura-builtin` provides the initial command and `graph-dag` view components.
- It is linked into the editor binary and enabled in the bundled default config.

## Non-goals / avoided approaches

- Global plugin hooks that mutate host behavior without config arbitration
- Runtime dynamic ABI plugins in v1
- Extension config UI or remote marketplace in v1
- Encoding kind in component ids (no `.command` / `.view` suffixes)
- Pulling editor GUI or extension loading into aura-engine
- Implementing `document-type` in the v1 skeleton

## Later

1. **`document-type` kind** — match/open/load traits; retire hardwired Imp-only open; GraphLibrary as a second type.
2. **Open pipeline** — path/bytes → document-type → session document → default view by role.
3. **Richer MDI** — multiple views per document, splits/panels, inactive-tab keep-alive, session restore.
4. **More extensions** — additional views/commands as separate crates linked into the binary.
5. **Project-level `extensions.json`** — load from a content/project root, not only bundled defaults.
6. **Engine-facing kinds** (if ever) — live in engine or a shared interfaces package; do not pull GUI into aura-engine.
7. **Dynamic loading** — only if compile-time link becomes insufficient; packaging decision, not the v1 contract.
