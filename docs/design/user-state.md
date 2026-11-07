# User state

Declarative requirements for ephemeral Aura Editor user-state persistence. Product context: [`product.md`](product.md). Extensibility: [`extensions.md`](extensions.md).

## Role

- The editor persists **ephemeral user state** as versioned JSON snapshots under a configurable local Aura user directory.
- This is ordinary durable local state (open tabs, future UI preferences written by the app). It is not a start/finish “session,” and it is not Imp document content.
- State is organized into **buckets** (one JSON file per bucket). Extension `user-state` components declare which bucket they belong to.

## User directory

- Default: `{working-directory}/.aura-user` (contained under the process CWD).
- Override with environment variable `AURA_USER_DIR` (absolute or relative path).
- Tests and tooling may pass an explicit directory; helpers that must not touch disk use an in-memory store.
- The directory is created on first successful write.

## Buckets and schema

- Each bucket id maps to `{user_dir}/{bucket}.json`.
- Bucket document version **1** fields: `version` (number) and `entries` (object).
- Each enabled `user-state` component owns `entries[component.id]`.
- Domain naming for buckets when practical. Convention: an extension that contributes its own state uses its extension id as the bucket id.
- Builtin open-document tabs use bucket `documents` and entry `open-documents`.
- Shell pane-grid layout uses bucket `shell` and entry `shell-layout`.
- Missing file, missing entry, unsupported version, or invalid JSON → defaults for that bucket/entry; the app still starts (with logging).

### Example (`documents.json`)

```json
{
  "version": 1,
  "entries": {
    "open-documents": {
      "openPaths": ["/abs/a.json", "/abs/b.json"],
      "activePath": "/abs/b.json"
    }
  }
}
```

## Persistence mechanics

- Updates apply to in-memory state immediately and mark the affected bucket dirty.
- Writes are **debounced** (500 ms idle) so bursts of UI changes coalesce into one write per generation token.
- Writes use **atomic replacement**: write a complete snapshot to a temporary sibling file, flush, then rename over the target.
- Do not serialize internal UI widget objects wholesale; persist versioned, purpose-built entry shapes.

## Extension kind `user-state`

- Linked extensions register user-state implementations; config enables them with `kind: "user-state"`.
- Required component param: `bucket` (string bucket id).
- Component `id` is the entry key inside that bucket.
- Host adapters own typed collect/apply logic so extension crates do not depend on `aura_editor` (same split as views).

## Open documents (first consumer)

- Entry shape: ordered `openPaths` plus optional `activePath`.
- Collect after open, activate, and close; apply once at startup by reopening paths through the normal document open pipeline, then selecting the active path when still open.
- Only documents with a real filesystem path are collected; untitled notation tabs are omitted.
- Paths that fail to open are skipped with a warning; startup is not blocked.
- Canvas pan/zoom is not persisted (see product non-goals).

## Piano roll UI

- Extension `aura-notation` contributes entry `piano-roll-ui` in bucket `aura-notation`.
- Entry shape: `activeTool` (`draw` | `select` | `erase`), `snapEnabled` (bool), `snapResolution` (`1/4` | `1/8` | `1/16` | `1/32`).
- Defaults when missing: draw tool, snap on, resolution `1/16`.
- Collect on tool, snap toggle, and snap-resolution changes; apply once at startup when the component is enabled.

## Graph DAG UI

- Extension `aura-builtin` contributes entry `graph-dag-ui` in bucket `aura-builtin`.
- Entry shape: `activeTool` (`select` | `add` | `connect` | `erase`), `connectionMode` (`simple` | `precise`), `addNodeType` (string node type id).
- Defaults when missing: select tool, simple connection mode, `addNodeType` `sine`.
- Collect on tool, connection-mode, and add-type changes; apply once at startup when the component is enabled.
- Selection and in-progress connect state are session-only (not persisted). Prefs are global across all graph tabs.

## Shell layout

- Extension `aura-builtin` contributes entry `shell-layout` in bucket `shell`.
- Entry shape: `version` (number, currently `1`) plus a recursive `tree` of either `{ "pane": "<id>" }` or `{ "split": { "axis": "horizontal" | "vertical", "ratio": number, "a": <tree>, "b": <tree> } }`.
- Pane ids: `status`, `view-toolbar`, `documents`. A valid tree contains each id exactly once.
- Default when missing/invalid: horizontal (top/bottom) stack status → view-toolbar → documents, with documents taking most of the height.
- Collect on pane drag-rearrange and split resize; apply once at startup when the component is enabled.
- Do not persist iced widget objects; only the purpose-built tree. Closing or hiding panes is out of scope.

### Example (`shell.json`)

```json
{
  "version": 1,
  "entries": {
    "shell-layout": {
      "version": 1,
      "tree": {
        "split": {
          "axis": "horizontal",
          "ratio": 0.08,
          "a": { "pane": "status" },
          "b": {
            "split": {
              "axis": "horizontal",
              "ratio": 0.1,
              "a": { "pane": "view-toolbar" },
              "b": { "pane": "documents" }
            }
          }
        }
      }
    }
  }
}
```

## Bindings relationship

- Keyboard `bindings.json` remains on the OS project config path (`directories::ProjectDirs`) and is hand-edited for now.
- A later change may relocate bindings under the Aura user directory or into a user-state bucket; that move is out of scope here.

## Non-goals / avoided approaches

- Calling this persistence layer a “session”
- Persisting pan/zoom camera state
- Writing or relocating `bindings.json` in this slice
- Settings UI for user state
- Project-level (content-root) user state distinct from the process user directory
