# Miro pull (editor host)

Declarative requirements for the editor’s Miro app-memory path. Product shell: [`product.md`](product.md). Engine host: Aura engine `docs/design/miro.md`. Graph missing nodes: [`graph-dag.md`](graph-dag.md). User state: [`user-state.md`](user-state.md).

## Role

- Miro layers for Aura: **text files** (authoring SoT) → **SQLite projection** (Mir sync) → **app memory** (`ModelGraph` tabs) via host **pull**.
- The editor **must** provide a durable workflow: open a `MiroSession` for a content root (workspace), resolve a product **NodeId** through the Aura host index, pull the Mir blob, and materialize it through the stacked document-type pipeline into a session tab.
- Product open **must** be Miro-first: **Open Workspace…** then **Open…** (NodeId picker / Ctrl+O). **Open File…** remains a secondary filesystem escape hatch.

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
- Imp graphs keep edge topology when endpoints lack a local `Graph.nodes` entry; missing cards / typecheck errors behave as for any incomplete graph ([`graph-dag.md`](graph-dag.md)).
- On open (filesystem or Miro), the host **must** hydrate edge-referenced **notation** tip NodeIds found under the content root (graph parent / workspace) into nullary `external_score` proxies in the local map — without marking the tab dirty — so composition tips are not red when the tip file is present.
- Double-click **OpenNode** on a still-missing card **must** resolve that NodeId via Miro into the **active** tab’s local map (copy from an `imp-graph` tip, or a nullary `external_score` proxy for a notation tip). When a document view maps for the tip kind, the host **may** also open/focus the Miro tab.

## Non-goals (this slice)

- Full Mir record list/query browser beyond the host NodeId index picker
- Dirty-tab conflict UI when the projection changes
- Replacing demos / engine `load_graph` with Miro-only open
- Path-under-root as a product open key
