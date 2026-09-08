# Aura Editor

GUI frontend and DAW-like surface for [Aura](https://github.com/silentorb/aura-engine) — oriented as a music/audio designer.

## Status

Rust / [Iced](https://iced.rs/) hello-world skeleton with unit and headless functional tests. Product intent lives under [`docs/`](docs/).

## Development environment

1. Open `~/dev/aura-workbench` in Cursor.
2. Reopen in Container (Compose starts `engine` + `editor`; Cursor attaches to **engine**).
3. Work in `/workspaces/aura-workbench/.mnt/aura-editor` (this tree).
4. Build and test **in the editor service** (GUI system deps live there):

```bash
docker compose -f /workspaces/aura-workbench/.devcontainer/docker-compose.yml exec editor bash -lc 'cd /workspaces/aura-workbench/.mnt/aura-editor && cargo test'
```

## Commands

From the editor container, in this directory:

```bash
cargo build
cargo test
cargo clippy
cargo run
```

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Silent Orb.
