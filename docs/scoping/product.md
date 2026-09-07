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

Refined requirements live in [`../design/product.md`](../design/product.md).

## Open questions

- UI toolkit and application stack
- How far hand-crafting of data and configuration goes versus generation and graph-driven workflows
- How tightly the editor couples to aura-engine APIs versus file/CLI boundaries
