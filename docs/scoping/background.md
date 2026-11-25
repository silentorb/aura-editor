# Background

Cross-cutting context that informed Aura Editor’s design. Prefer linking from design docs rather than duplicating chronology.

## Separate repo from the engine

Aura splits the headless engine and the editor so the engine can ship and develop as an independent product. Embedding GUI code inside the engine repo would force that stack onto every engine consumer and blur product boundaries.

| Repo | Role |
|------|------|
| `aura-workbench` | Primary workspace; Compose/Dev Container; mounts siblings; pack assembly and export |
| `aura-engine` | Headless procedural composition and synthesis (Rust), plus the pack's extension libraries |
| `aura-editor` | This product — Aura's editor UI as Imp Lab extensions, and the Aura pack configuration |
| `imp-lab` | The domain-neutral host (Imp graphs, Tome records, extension system) |

## Designer, not traditional DAW

Traditional DAWs center on manually editing music and audio data (clips, takes, destructive or track-based editing). Aura’s spirit is procedural and declarative. The editor still needs ways for users to hand-craft data, but that support should reinforce design of generative systems rather than replace Aura with a conventional DAW workflow.

## Dev environment

Primary development is via **aura-workbench** (Dev Container + Compose). This repo is mounted at `/workspaces/aura-workbench/.mnt/aura-editor`; build and test it with Bun in the **imp-lab** service. Do not treat the editor repo root as the Dev Container entrypoint.
