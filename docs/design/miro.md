# Miro pull (editor host)

Declarative requirements for the editor’s Miro app-memory path. Product shell: [`product.md`](product.md). Engine host: Aura engine `docs/design/miro.md`. Graph missing nodes: [`graph-dag.md`](graph-dag.md). User state: [`user-state.md`](user-state.md).

## Role

- Miro layers for Aura: **text files** (authoring SoT) → **SQLite projection** (Mir sync) → **app memory** (`ModelGraph` tabs) via host **pull**.
- The editor **must** provide a durable workflow: open a `Session` for a content root (workspace), resolve a product **NodeId** through the Aura host index, pull the Mir blob, and materialize it through the stacked document-type pipeline into a session tab.
- Product open **must** be Miro-first: **Open Workspace…** then **Open…** (NodeId picker / Ctrl+O). **Open File…** remains a secondary filesystem escape hatch.

## Identity

- Product Miro open / resolve keys are Imp **NodeId**s (`open_pulled_node` / `open_pulled_node_tab`).
- Aura maintains a host index over the content root: `imp-graph` local node ids and `notation` tip `id` map to opaque Mir record ids.
- Mir record ids **may** be path-shaped text-store keys internally; they are **not** the product API and **must not** be joined from a NodeId.

## Document origin

- Every non-untitled tab records a `DocumentOrigin`: `Filesystem { path }` or `Pulled { content_root, node_id, mir_record_id }`.
- Filesystem opens dedupe by path. Miro opens dedupe by `(content_root, mir_record_id)` so multiple NodeIds in the same blob share one tab.
- Miro tabs expose an authoring path derived from the Mir record id (not from the NodeId) for Save and library-neighbor hints. Save writes the text SoT; it does not write SQLite as authoring truth.

## Pull semantics

- Pull is host index resolve → `poll_watch` → `pull_record(mir_record_id)` → `open_document_bytes` → `ModelGraph` (same stack as filesystem bytes).
- Imp graphs keep edge topology when endpoints lack a local `Graph.nodes` entry; missing cards / typecheck errors behave as for any incomplete graph ([`graph-dag.md`](graph-dag.md)).
- On open (filesystem or Miro), edge-referenced NodeIds that the content-root Miro index can resolve **must** be treated as host remotes (not red missing cards) without inserting Imp proxy nodes into the local map. Remote cards present host-resolved title and type like local cards.
- Double-click **OpenNode** on a missing/remote card **must** resolve via Miro: copy an `imp-graph` node into the active local map, or open/focus a notation tip tab without inserting a fake Imp node.

## Non-goals (this slice)

- Full Mir record list/query browser beyond the host NodeId index picker
- Dirty-tab conflict UI when the projection changes
- Replacing demos / engine `load_graph` with Miro-only open
- Path-under-root as a product open key
