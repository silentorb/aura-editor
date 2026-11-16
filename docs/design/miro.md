# Miro pull (editor host)

Declarative requirements for the editor’s parallel Miro app-memory path. Product shell: [`product.md`](product.md). Engine host: Aura engine `docs/design/miro.md`. Graph missing nodes: [`graph-dag.md`](graph-dag.md).

## Role

- Miro layers for Aura: **text files** (authoring SoT) → **SQLite projection** (Mir sync) → **app memory** (`ModelGraph` tabs) via host **pull**.
- The editor **must** provide a durable programmatic workflow: open a `MiroSession` for a content root, resolve a product **NodeId** through the Aura host index, pull the Mir blob, and materialize it through the stacked document-type pipeline into a session tab.
- File → **Open…** **must** remain the filesystem open path. Miro open is **not** menu-attached in this slice (intentional gap; later UI / node-ref navigation calls the same API).

## Identity

- Product Miro open / resolve keys are Imp **NodeId**s (`open_miro_node` / `open_miro_node_tab`).
- Aura maintains a host index over the content root: `imp-graph` local node ids and `notation` tip `id` map to opaque Mir record ids.
- Mir record ids **may** be path-shaped text-store keys internally; they are **not** the product API and **must not** be joined from a NodeId.

## Document origin

- Every non-untitled tab records a `DocumentOrigin`: `Filesystem { path }` or `Miro { content_root, node_id, mir_record_id }`.
- Filesystem opens dedupe by path. Miro opens dedupe by `(content_root, mir_record_id)` so multiple NodeIds in the same blob share one tab.
- Miro tabs expose an authoring path derived from the Mir record id (not from the NodeId) for Save and library-neighbor hints. Save writes the text SoT; it does not write SQLite as authoring truth.

## Pull semantics

- Pull is host index resolve → `poll_watch` → `pull_record(mir_record_id)` → `open_document_bytes` → `ModelGraph` (same stack as filesystem bytes).
- Imp graphs keep edge topology when endpoints lack a local `Graph.nodes` entry; missing cards / typecheck errors behave as for any incomplete graph ([`graph-dag.md`](graph-dag.md)). Resolving those ids by a further Miro pull is later host policy.

## Non-goals (this slice)

- File-menu Miro workspace / record pickers
- Mir record list/query browser
- Restoring Miro-origin tabs via pull on startup (filesystem `openPaths` only)
- Dirty-tab conflict UI when the projection changes
- Replacing demos / engine `load_graph` with Miro-only open
