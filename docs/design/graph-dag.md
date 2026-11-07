# Graph DAG view

Declarative requirements for the Imp `graph-dag` view: visualization, modal tools, and connection editing. Product context: [`product.md`](product.md). User-state: [`user-state.md`](user-state.md).

## Role

- The `graph-dag` view is the primary surface for editing single-graph Imp documents (`imp-graph`).
- Users create, select, rename, retype, and delete nodes, and create or delete connections, on an automatically laid-out Canvas.
- Layout remains automatic (petgraph layered). Edits regenerate layout. Freeform node placement is not supported.

## Implementation crate

- Canvas, layout, pure edit helpers, UI prefs state, and lightweight toolbar/properties **building blocks** live in the Aura-free workspace crate `imp_graph_editor`.
- Aura Editor hosts compose those blocks into the product toolbar, Properties pane, menus, commands, Aura JSON save format, and Aura node-type registry wiring.
- `imp_graph_editor` must not depend on `aura_*` crates, so a future extract into a separate Imp editor repo stays mostly packaging.

## Tools

- Modal tools live on the per-view toolbar: **Select**, **Add**, **Connect**, **Erase**.
- Tool choice is global across all open graph tabs (see user-state `graph-dag-ui`).
- Default keyboard bindings when a graph tab is active: `S` Select, `A` Add, `C` Connect, `E` Erase. **Delete** removes the current selection in Select.

| Tool | Behavior |
|------|----------|
| Select | Click a node or edge to select; click empty space clears selection. With a node selected, the Properties pane exposes rename (id) and change type. Middle-button or empty-space drag pans; wheel zooms. |
| Add | Click empty canvas inserts a node of the current Add type with a unique id; layout regenerates. |
| Connect | Creates an edge according to the active connection mode (below). |
| Erase | Click a node removes it and all incident edges. Click an edge removes that edge; in **Simple** mode a visual node→node line removes all edges between that pair. |

## Connection modes

- **Simple**: edges are drawn node-to-node (ports ignored for display). Connecting clicks a source node then a target node; the editor auto-pairs an unused compatible output→input from the node-type registry. If no pair exists, the action fails with a status message.
- **Precise**: nodes show labeled port lists — input ports (left) and output ports (right), each as a circle with its port id beside it. Connecting drags from an output handle to an input handle. Edge endpoints attach to the chosen ports. Node height grows with `max(inputs, outputs)` when ports are shown.
- Connection mode is global across graph tabs and is toggled from **Graph view options** via a **Precise ports** checkbox (checked = Precise, unchecked = Simple).

### Auto-pair rules (Simple)

1. Prefer unused target input ports (no existing edge into that port).
2. Prefer output→input pairs whose signal types are compatible when type information is available.
3. Otherwise take the first remaining free output→input pair.
4. Fail if none remain.

## Toolbar

```
[Select] [Add ▾] [Connect] [Erase]     <space>     [Graph view options ▾]
```

- **Add ▾** lists node types from the playback registry; choosing a type sets the Add type and activates Add.
- **Graph view options ▾** (far right) holds global view prefs. First control: **Precise ports** checkbox toggle. More options may join later without relocating the control.

## Selection and update

- Selection is session-only (not persisted).
- **Update** in this slice means rename node id and change node type for the selected node in the shell **Properties** pane (not the view toolbar). A full input-literal property inspector is out of scope.

## Persistence of documents

- Users may create a new empty Imp graph (untitled), edit it, and save Aura JSON with `type: "imp-graph"` plus `nodes` / `edges` (Save As when untitled).
- Opened graphs are editable in memory; Save writes the same format. Playback continues to use the in-memory graph without requiring a save.

## Non-goals / avoided approaches

- Freeform manual node placement or persisted layout positions
- Per-document connection mode or tool choice
- Full input-literal / property inspector for node inputs
- Undo/redo
- Opening GraphLibrary catalogs as editable documents

Background: [`../scoping/product.md`](../scoping/product.md).
