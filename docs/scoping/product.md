# Product scoping

Motivation and goals for Aura Editor — Aura's editor UI, a music/audio designer surface.

## Vision

Aura Editor is the graphical side of Aura: a DAW-like environment oriented toward designing music and audio procedurally and declaratively, rather than toward traditional manual editing of stored music and audio data.

It stays in its own repository so **aura-engine** can remain an independent, headless solution. Users who only need the engine should not pull editor UI.

## Goals

- Give users a GUI to work with Aura without abandoning the procedural, declarative spirit of the engine
- Support hand-crafting data where that helps design, without making destructive timeline editing the primary model
- Position the product as a music/audio **designer**, distinct from a traditional DAW
- Treat **Imp graphs** as the primary persistent resources, with hand-crafted records (notation, and later others) as complements
- Spread Aura content across many small records composed like a programming project, rather than one binary project file
- Provide a hand-crafted **piano roll** for notation, useful when fully procedural authoring is too hard

## Built on Imp Lab

Imp graphs are not Aura-specific: Imp is a general graph language, and **Imp Lab** is its general tool — projects of Tome data stores, records, the graph editor, extensions. Aura Editor is what makes that tool Aura's editor. The plan was always that Imp Lab stays free of audio concepts and all audio-specific UI lives here; the piano roll was for a time built directly into Imp Lab, which was a mistake corrected by moving it into this repo as an extension.

So Aura Editor is a set of modular extension components rather than its own application:

- The piano roll is a `node-type-editor` implementation, wired by configuration to the Notations node type.
- Key bindings, menu items, settings and node UI are components in the Aura pack's `extensions.json`, which this repo owns.
- When Aura's UI needs something the host can't do yet, Imp Lab gains a domain-neutral extension capability (that's how editors for extension node types came about) rather than Aura code.

Tome made "documents" unnecessary: every record is a Tome node, and what it is comes from its node type. Aura's record kinds are Tome node types, not document types.

The earlier editor in this repo was a Rust / Iced application hosting Imp Lab's Iced graph-editor crate. Both were removed; there is one host (Imp Lab, Tauri + React) and Aura's UI plugs into it.

Refined requirements: [`../design/product.md`](../design/product.md), [`../design/pack.md`](../design/pack.md), [`../design/piano-roll.md`](../design/piano-roll.md).

## Open questions

- Which further Aura record kinds deserve their own editors (instrument definitions, arrangements)
- Whether Aura UI beyond editors (transport bar, mixer views) needs new UI component kinds in Imp Lab

## Resolved

- Notation records are first-class `score` sources inside Imp graphs, referenced by node id and resolved from the project's data stores.
- Play from a notation tab uses the open notation (with the parked playhead), not a lookup of a saved file.
