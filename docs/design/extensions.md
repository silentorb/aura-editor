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
- The host content pane resolves a document’s view by walking the open document’s type chain from tip to root and taking the first declared role that has an enabled view.
- Imp single-graph tips use role `imp-graph`. The builtin `graph-dag` view claims that role. Role `json` is reserved for a future generic JSON fallback view.
- Iced `Element` construction may live in a host adapter so `aura_interfaces` does not depend on application `Message` types. View packages may still depend on Iced for their own widgets.
- When no enabled view implements any role on the chain, the host shows an explicit error placeholder (not a panic).
- When multiple views claim the same role, the first enabled config claim wins.

### `document-type`

- Document types are **stackable**: each has an optional parent id. Roots match raw file bytes; children refine a parent payload.
- Open pipeline: path/bytes → stacked document-types → session document (full chain + tip payload) → view by role (tip first, then parents).
- Builtin stack for graphs: `json` → `aura-json` → `imp-graph` (success) or `imp-graph-library` (open rejects; no session tab).
- **Aura JSON:** a JSON object with root string property `type`. After `aura-json` opens, the host selects the child whose implementation id equals that `type` value (no sibling sniffing).
- Payload between layers is type-erased (`Any`); Imp `Graph` deserialization lives in the `imp-graph` implementation.

## Builtin extension

- Package id `aura-builtin` provides commands, stackable document types (`json`, `aura-json`, `imp-graph`, `imp-graph-library`), and the `graph-dag` view.
- It is linked into the editor binary and enabled in the bundled default config.

## Non-goals / avoided approaches

- Global plugin hooks that mutate host behavior without config arbitration
- Runtime dynamic ABI plugins in v1
- Extension config UI or remote marketplace in v1
- Encoding kind in component ids (no `.command` / `.view` suffixes)
- Pulling editor GUI or extension loading into aura-engine
- A separate `content-type` kind (generality comes from document-type stacking)
- Opening GraphLibrary catalogs as session documents until that flow is designed

## Later

1. **Richer MDI** — multiple views per document, splits/panels, inactive-tab keep-alive, session restore.
2. **More extensions** — additional views/commands/document types as separate crates linked into the binary.
3. **JSON fallback view** — enable a view for role `json` when a more specific view is missing.
4. **Project-level `extensions.json`** — load from a content/project root, not only bundled defaults.
5. **Engine-facing kinds** (if ever) — live in engine or a shared interfaces package; do not pull GUI into aura-engine.
6. **Dynamic loading** — only if compile-time link becomes insufficient; packaging decision, not the v1 contract.
