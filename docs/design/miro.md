# Miro pull (editor host)

Declarative requirements for the editor’s parallel Miro app-memory path. Product shell: [`product.md`](product.md). Engine host: Aura engine `docs/design/miro.md`. Graph missing nodes: [`graph-dag.md`](graph-dag.md).

## Role

- Miro layers for Aura: **text files** (authoring SoT) → **SQLite projection** (Mir sync) → **app memory** (`ModelGraph` tabs) via host **pull**.
- The editor **must** provide a durable programmatic workflow: open a `MiroSession` for a content root, pull a record by opaque id, and materialize it through the stacked document-type pipeline into a session tab.
- File → **Open…** **must** remain the filesystem open path. Miro open is **not** menu-attached in this slice (intentional gap; later UI / node-ref navigation calls the same API).

## Document origin

- Every non-untitled tab records a `DocumentOrigin`: `Filesystem { path }` or `Miro { content_root, record_id }`.
- Filesystem opens dedupe by path. Miro opens dedupe by `(content_root, record_id)`.
- Miro tabs still expose a derived **authoring path** (`content_root` + record id segments) for Save and library-neighbor hints. Save writes the text SoT; it does not write SQLite as authoring truth.

## Pull semantics

- Pull is `poll_watch` → `pull_record` → `open_document_bytes` → `ModelGraph` (same stack as filesystem bytes).
- Imp graphs keep edge topology when endpoints lack a local `Graph.nodes` entry; missing cards / typecheck errors behave as for any incomplete graph ([`graph-dag.md`](graph-dag.md)). Resolving those ids by a further Miro pull is later host policy.

## Non-goals (this slice)

- File-menu Miro workspace / record pickers
- Mir record list/query browser
- Restoring Miro-origin tabs via pull on startup (filesystem `openPaths` only)
- Dirty-tab conflict UI when the projection changes
- Replacing demos / engine `load_graph` with Miro-only open
