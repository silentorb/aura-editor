# Product scoping

Motivation and goals for Aura Editor — the GUI frontend and music/audio designer surface for Aura.

## Vision

Aura Editor is a graphical front end for Aura: a DAW-like environment oriented toward designing music and audio procedurally and declaratively, rather than toward traditional manual editing of stored music and audio data.

It stays in its own repository so **aura-engine** can remain an independent, headless solution. Users who only need the engine should not pull editor UI or OS-integration stacks.

## Goals

- Give users a GUI to work with Aura without abandoning the procedural, declarative spirit of the engine
- Support hand-crafting data and configuration where that helps design, without making destructive timeline editing the primary model
- Carry GUI and OS-integration dependencies that the headless engine should not need, while likely sharing most of the engine’s core dependency set
- Position the product as a music/audio **designer**, distinct from a traditional DAW
- Treat **Imp graphs** as the primary persistent resources (flat, orthogonal UX), rather than traditional DAW “projects” and supporting resource trees
- Spread Aura content across multiple plain-text files (graphs, notation, and later other resources), composed like a programming project rather than crammed into a single binary project file
- Provide a hand-crafted **piano roll** for notation as a complement to procedural generation—useful when fully procedural authoring is too hard—without making destructive timeline editing the center of the product
- Provide a multi-document interface so users can open multiple documents (graphs and notation) in tabs

## Application stack

The editor is a **Rust** desktop application using **[Iced](https://iced.rs/)** as the GUI framework. Day-to-day development runs in WSL Linux containers via **aura-workbench**. The intended end-user environment is **Windows**. Workbench provides an on-demand Windows MSVC cross-compile service for shipping the editor binary.

Refined requirements live in [`../design/product.md`](../design/product.md), [`../design/extensions.md`](../design/extensions.md), [`../design/input-bindings.md`](../design/input-bindings.md), and [`../design/testing.md`](../design/testing.md). Extension motivation: [`extensions.md`](extensions.md).

## Graphs and files

Aura JSON files declare a root `type` and stack through document types (`json` → `aura-json` → …). Single-graph Imp files (`type: "imp-graph"` plus `nodes` / `edges`) are the first tip users open successfully. Multi-graph catalogs (`type: "imp-graph-library"`) are recognized then rejected until catalog open is designed.

Hand-crafted notation lives in separate Aura JSON files (`type: "notation"`) with beat-grid note events. Users create, open, and save those documents in a piano-roll view (modal Draw / Select / Erase tools). Notation events are the same engine `Score` material procedural generators produce; Play from a piano-roll tab auditions them through the Imp sine voice path with a user-placed playhead.

Open Imp graphs show an automatic Canvas DAG view (nodes, ids/types, edges) with pan and zoom. Users edit nodes and connections with modal Select / Add / Connect / Erase tools; connection UI toggles between simple node→node wiring and precise port handles. Node layout is automatic only — no freeform dragging of nodes on a canvas. Users may create and save Imp graph documents as well as notation. Notation documents open into the piano-roll view. Start/Stop/Toggle (Space) streams the active Imp graph tab through CPAL (from `t = 0`) or auditions the active notation tab from the parked playhead; saving is not required to play.

Refined graph-view requirements: [`../design/graph-dag.md`](../design/graph-dag.md).

## Input and commands

Users need a configurable layer between raw input and editor actions so shortcuts (and later other input) can be remapped without hard-coding keystrokes in UI code. Keyboard is the first source; the same command ids should stay usable from global menus, per-view toolbars, and from a future bindings editor. Prefer hand-editable JSON (user file overriding a bundled default) ahead of building that UI. The app should still run when no bindings file is present. Native OS input commands (clipboard, quit, window manager, and similar) stay outside this layer.

Shell chrome splits global menus (File / Edit / View / Play) from per-view toolbars (for example piano-roll tools). A global toolbar and contextual per-view menus may come later.

Refined requirements: [`../design/input-bindings.md`](../design/input-bindings.md).

## Open questions

- Whether notation files later appear as first-class `score` sources inside user Imp graphs (beyond editor audition bindings)
- How tightly the editor couples to aura-engine APIs versus file/CLI boundaries
- Whether everyday authoring stays 1:1 graph-per-file or also embraces GraphLibrary catalogs as first-class multi-document sources
