# Aura Editor

GUI frontend and DAW-like surface for [Aura](https://github.com/silentorb/aura-engine) — oriented as a music/audio designer.

## Status

**Deprecated (Iced).** This Iced Aura Editor is deprecated and will be removed once the Imp Lab Tauri editor path stabilizes. **Do not add new Iced editor features** here.

The active editor path is **[imp-lab](https://github.com/silentorb/imp-lab)** Tauri (`apps/tauri/`) plus Aura’s optional Lab pack (`aura-engine/lab-pack` + `aura_imp_lab_catalogs` extension). Product intent for the Iced shell remains under [`docs/`](docs/) until removal.

Historical note: Rust / [Iced](https://iced.rs/) multi-document shell hosted **imp-lab** `imp_graph_editor` with Aura JSON, playback, and extension wiring (see [`docs/design/extensions.md`](docs/design/extensions.md)).

## Development

Multi-repo Docker Compose orchestration lives in **aura-workbench**. From this repo root (with GUI system deps available):

```bash
cargo build
cargo test
cargo clippy
cargo run
```

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Silent Orb.
