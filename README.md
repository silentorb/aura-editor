# Aura Editor

Aura's editor UI — a music/audio designer surface for [Aura](https://github.com/silentorb/aura-engine) — delivered as extensions for [Imp Lab](https://github.com/silentorb/imp-lab).

Imp Lab is the domain-neutral host (projects of Tome data stores, Imp graph editing, an extension system). Everything audio-specific a user sees lives here, as modular components that the Aura pack's configuration wires into Lab:

- `src/` — the UI library (`dist/aura_editor.js`): implementations by id, currently the **notation editor** (piano roll with playback, note audition and recording takes). See [`docs/design/piano-roll.md`](docs/design/piano-roll.md).
- `pack/extensions.json` — the **Aura pack** configuration: aura-engine's catalog and playback libraries plus this UI library, with their commands, menus, settings, node UI, editors and key bindings. See [`docs/design/pack.md`](docs/design/pack.md).

What ships is `imp-lab.exe` with the assembled Aura pack beside it (built by aura-workbench).

## Development

Multi-repo orchestration lives in **aura-workbench**; use its **imp-lab** service (Bun). From this repo root:

```bash
bun install
bun run test
bun run build      # typecheck + dist/aura_editor.js
```

Assemble a local pack and run Imp Lab with it from the workbench: `scripts/assemble-aura-pack.sh`, then `IMP_LAB_EXTENSIONS=/workspaces/aura-workbench/.aura-pack/extensions.json`.

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Silent Orb.
