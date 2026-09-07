# Background

Cross-cutting context that informed Aura Editor’s design. Prefer linking from design docs rather than duplicating chronology.

## Separate repo from the engine

Aura splits the headless engine and the editor so the engine can ship and develop as an independent product. Embedding a large GUI and OS-integration surface inside the engine repo would force that stack onto every engine consumer and blur product boundaries.

| Repo | Role |
|------|------|
| `aura-workbench` | Primary workspace; Compose/Dev Container; mounts siblings |
| `aura-engine` | Headless procedural composition and synthesis (Rust) |
| `aura-editor` | This product — GUI frontend / designer surface |

## Dependency expectations

The editor will probably share most of the same dependencies as aura-engine (composition, synthesis, graph, and related concerns), plus eventually a large additional set for GUI and OS integration that the engine does not need. Keeping those extras out of the engine tree is a primary reason for the split.

## Designer, not traditional DAW

Traditional DAWs center on manually editing music and audio data (clips, takes, destructive or track-based editing). Aura’s spirit is procedural and declarative. The editor still needs ways for users to hand-craft data and configuration, but that support should reinforce design of generative systems rather than replace Aura with a conventional DAW workflow.

## Dev environment

Primary development is via **aura-workbench** (Dev Container + Compose). This repo is mounted at `/workspaces/aura-workbench/.mnt/aura-editor`. Do not treat the editor repo root as the Dev Container entrypoint.
