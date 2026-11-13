# Aura Editor

GUI frontend and DAW-like surface for [Aura](https://github.com/silentorb/aura-engine) — oriented as a music/audio designer.

## Status

Rust / [Iced](https://iced.rs/) multi-document shell: open single Imp graph files into tabs and view them as an automatic Canvas DAG (pan/zoom; no freeform node placement). The Imp graph-dag surface and chrome building blocks live in **imp-lab**’s `imp_graph_editor` crate; this package hosts them with Aura JSON, playback, and extension wiring. Commands and views register as extension components (see [`docs/design/extensions.md`](docs/design/extensions.md)). Product intent lives under [`docs/`](docs/).

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
