# Documentation

This directory is the source of truth for Aura Editor. Design docs define what the software must be (declarative requirements); code must conform to them. Scoping holds motivation and context that feed design.

## Docs govern code

When documentation and implementation disagree:

1. Update the code to match the doc, or
2. Explicitly revise the doc to reflect an intentional change.

Never let code silently drift from documented design.

## Scoping vs design

Follow [Silent Orb: Scoping and Design](https://silentorb.com/business/product/scoping-and-design/):

| Mode | Directories | Purpose |
|------|-------------|---------|
| **Scoping** | [`scoping/`](scoping/) | Motivation, goals, background, inspiration, context. Optimized for **writing** (low friction). May stay loosely organized. |
| **Design** | [`design/`](design/) | **Declarative requirements** that define the product. Optimized for **reading** and day-to-day agent use. |

- Prefer gather (scoping) then refine (design). When they overlap, switch modes consciously.
- Design may **link to scoping** for motivation origins without dumping chronology into design.
- There is no separate “requirements” tree and no append-only decision log. Design *is* the requirements surface.

## Directory map

| Path | Purpose |
|------|---------|
| [`scoping/`](scoping/) | Cross-cutting motivation and context |
| [`design/`](design/) | Cross-cutting declarative product definition |

## When to update

Update docs when:

- Adding or changing a feature
- Changing observable behavior
- Settling or reversing an architectural direction
- Receiving a durable agent or user instruction that defines persistent project intent

## Authoring style

- Write design **declaratively and timelessly** (what is true now).
- Prefer updating an existing design doc over creating a new historical record.
- When reversing a direction: add a short **avoided approach** in design; optionally note background in scoping; remove obsolete prescriptions.
- Do not duplicate content from the root [`AGENTS.md`](../AGENTS.md) — link to it instead.
- Prefer updating an existing doc over creating a new one unless the topic warrants separation.

## Agent capture rules

| Capture this | Write here |
|--------------|------------|
| Motivation, goals, intent, inspiration, background | `docs/scoping/` |
| Declarative musts, non-goals, avoided approaches | `docs/design/` |
| One-off task notes | Nowhere durable |

Plans that change durable intent must include an explicit scoping and/or design doc update.
