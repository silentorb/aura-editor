# Graph DAG view

Declarative requirements for the Imp `graph-dag` view: visualization, always-on pointer editing, and connection modes. Product context: [`product.md`](product.md). User-state: [`user-state.md`](user-state.md).

## Role

- The `graph-dag` view is the primary surface for editing single-graph Imp documents (`imp-graph`).
- Users create, select, retitle, and delete nodes, and create or delete connections, on an automatically laid-out Canvas. Changing an existing node's type is a secondary concern (not a primary Properties action in this slice).
- Layout remains automatic (petgraph layered). Edits regenerate layout. Freeform node placement is not supported.

## Implementation crate

- Canvas, layout, pure edit helpers, UI prefs state, and lightweight toolbar/properties **building blocks** live in the Aura-free `imp_graph_editor` crate in **imp-lab**.
- Multi-document session bookkeeping (tabs, active document, path/dirty lifecycle) lives in imp-lab’s Aura-free `imp_documents` backend crate; Aura supplies host payloads (including notation-backed `ModelGraph`) and stacked open/save.
- Aura Editor depends on those crates and hosts compose UI blocks into the product toolbar, Properties pane, menus, commands, Aura JSON save format, and Aura node-type registry wiring.
- `imp_graph_editor` and `imp_documents` must not depend on `aura_*` crates.

## Pointer model (no modal tools)

Graph editing does **not** use mutually exclusive modal tools (unlike the piano-roll). Selection and wiring are always available; Add is a discrete command; delete is keyboard-on-selection.

| Action | Behavior |
|--------|----------|
| Select | Click a node or edge to select; empty-space drag pans (selection clear on empty click is not required when pan wins). With a node selected, the Properties pane shows Title / Id / Type in a two-column properties view (editable title auto-commits after a brief idle pause, or immediately on Enter; id and type read-only). Middle-button drag pans; wheel zooms. |
| Add node | Command `graph.add.node` (toolbar **Add node** button; key `A` when a graph tab is active) opens a modal to choose a node type. Submit inserts a node of that type with a new ULID id, a unique title derived from the type (`sine`, `sine_2`, …), and regenerates layout; Cancel/Escape closes with no edit. Last-used type is remembered in user state. |
| Connect | Always-on drag-to-connect per connection mode (below), with a rubber-band preview while dragging. Not a toolbar tool. |
| Disconnect port | In **Precise** mode, **right-click** a port handle to remove every edge attached to that port. |
| Delete | **Delete** / **Backspace** removes the current selection (node or edge). There is no Erase tool or click-to-erase. |

## Connection modes

- **Simple**: edges are drawn node-to-node (ports ignored for display). Press on a source node and drag to a target node; a rubber-band line follows the cursor. Release on another node auto-pairs an unused compatible output→input from the node-type registry. Release elsewhere (or without enough movement) cancels the connect and a short press selects. If no pair exists, the action fails with a status message.
- **Precise**: nodes show labeled port lists — input ports (left) and output ports (right), each as a circle with its port id beside it. Press an output handle and drag to a compatible input handle (or variadic dummy); a rubber-band line follows the cursor until release. Release on a valid input completes the edge; release elsewhere cancels. Node body / edge clicks select. **Right-click** a port handle disconnects all edges on that port (dummy handles are no-ops). Node height grows with `max(inputs, outputs)` when ports are shown. Automatic layered layout spaces same-rank nodes and content bounds using those rendered sizes (not the simple-mode box).
- Connection mode is global across graph tabs and is toggled from **Graph view options** via a **Precise ports** checkbox (checked = Precise, unchecked = Simple).

### Auto-pair rules (Simple)

1. Prefer unused **singleton** target input ports (no existing edge into that port).
2. Prefer output→input pairs whose signal types are compatible when type information is available.
3. Otherwise take the first remaining free singleton output→input pair.
4. If only **variadic** inputs remain available, append another edge to a compatible variadic port (assigning the next `Edge.order`).
5. Fail if none remain.

### Variadic inputs (Precise)

- Catalog ports marked `variadic` expand into one left-side handle per inbound edge (sorted by `Edge.order`), plus a trailing empty **dummy** handle for appending a new connection.
- Precise connect targets the dummy (append) or a singleton input; filled variadic handles are not connect targets.
- New edges into a variadic port store a unique `order` (`0..n-1` contiguous after reorder).
- In Precise mode, drag a filled variadic handle vertically among its siblings to reorder; on drop, `Edge.order` values are rewritten. The dummy is not reorderable.

## Toolbar

```
[Add node]     <space>     [Graph view options ▾]
```

- **Add node** runs `graph.add.node` (opens the type modal).
- **Graph view options ▾** (far right) holds global view prefs. First control: **Precise ports** checkbox toggle. More options may join later without relocating the control.

## Selection and update

- Selection is session-only (not persisted).
- **Update** in this slice means edit the selected node’s title in the shell **Properties** pane (not the Documents tool strip): Title, Id, and Type appear as rows in a reusable two-column properties view (label | value). Title edits apply after a short typing idle (or immediately on Enter); there is no separate Rename button. Id and type are shown read-only; changing type is deferred to a later secondary flow. A full input-literal property inspector is out of scope.

## Persistence of documents

- Users may create a new empty Imp graph (untitled), edit it, and save Aura JSON with `type: "imp-graph"` plus `nodes` / `edges` (Save As when untitled).
- Opened graphs are editable in memory; Save writes the same format. Playback continues to use the in-memory graph without requiring a save.

## Non-goals / avoided approaches

- Freeform manual node placement or persisted layout positions
- Modal Select / Add / Connect / Erase tools on the graph toolbar (notation-style tool modality)
- Click-to-place Add or click-to-erase on the canvas
- Per-document connection mode
- Full input-literal / property inspector for node inputs
- Changing an existing node's type from the Properties pane (read-only type label; secondary flow later)
- Undo/redo
- Opening GraphLibrary catalogs as editable documents

Background: [`../scoping/product.md`](../scoping/product.md).
