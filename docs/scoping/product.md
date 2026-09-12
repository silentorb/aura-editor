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
- Provide a multi-document interface so users can open multiple graph documents in tabs

## Application stack

The editor is a **Rust** desktop application using **[Iced](https://iced.rs/)** as the GUI framework. Day-to-day development runs in WSL Linux containers via **aura-workbench**. The intended end-user environment is **Windows**. Workbench provides an on-demand Windows MSVC cross-compile service for shipping the editor binary.

Refined requirements live in [`../design/product.md`](../design/product.md) and [`../design/testing.md`](../design/testing.md).

## Graphs and files

Single-graph Imp files (`{ nodes, edges }`) are the first document type users open. Multi-graph catalogs use Imp **GraphLibrary** (`definitions[].body`); opening those from the editor comes later.

Open documents show an automatic Canvas DAG view (nodes, ids/types, edges) with pan and zoom. Node layout is automatic only — no freeform dragging of nodes on a canvas.

## Open questions

- How far hand-crafting of data and configuration goes versus generation and graph-driven workflows
- How tightly the editor couples to aura-engine APIs versus file/CLI boundaries
- Whether everyday authoring stays 1:1 graph-per-file or also embraces GraphLibrary catalogs as first-class multi-document sources
