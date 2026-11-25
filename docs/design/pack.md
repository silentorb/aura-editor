# Aura pack

Declarative requirements for the Aura pack: what Imp Lab loads to become Aura's editor. Product context: [`product.md`](product.md). Extension system: [Imp Lab extensions](https://github.com/silentorb/imp-lab/blob/main/docs/features/extensions.md).

## Ownership

- The pack configuration, [`../../pack/extensions.json`](../../pack/extensions.json), is owned by **aura-editor**. It wires every Aura extension: aura-engine's libraries and Aura Editor's UI library.
- aura-engine owns the backend libraries (`aura_imp_lab_catalogs`, `aura_imp_lab_playback`) and pack data (`lab-pack/catalogs/instruments.json`, `lab-pack/demo-project/`), and the ids they declare (service `playback`, topics such as `aura.midi.status`, `aura.capture.take`, `aura.plugins.*`).
- aura-editor owns its UI implementations (`notation-editor`) and the commands they handle (`notation.*`).

## Contents

| Extension | Library | Components |
| --- | --- | --- |
| `aura-imp-catalogs` | `extensions/aura_imp_lab_catalogs` (cdylib) | `imp-catalog` |
| `aura-playback` | `extensions/aura_imp_lab_playback` (cdylib) | `service` (`playback`); `command` `play.start` / `play.stop` / `play.toggle`; Play menu items; MIDI settings, category and status item; plugin node literal options and GUI actions; `binding` Space → `play.toggle` |
| `aura-editor` | `extensions/aura_editor.js` (UI library) | `node-type-editor` `aura.notation-editor` (Notations node type, column `notation`, document type `notation`, service `aura-playback`); `binding`s D / S / E / N / R for the notation tools, snap and record |

## Layout

```
extensions.json
extensions/
  aura_imp_lab_catalogs.{so,dll}
  aura_imp_lab_playback.{so,dll}
  aura_editor.js
catalogs/instruments.json
demo-project/           (never overwritten by a re-export: users edit it)
```

- The workbench assembles it: `scripts/assemble-aura-pack.sh` for a local Linux run (`IMP_LAB_EXTENSIONS=<pack>/extensions.json`), and the Windows export beside `imp-lab.exe`.
- [`../../pack/extensions.test.ts`](../../pack/extensions.test.ts) keeps the configuration consistent: components reference declared extensions and services, the MIDI and plugin components use the engine's ids, the notation editor names an exported implementation, and bindings name commands the pack or the editor handles.

## Sample project

The demo project is seeded from aura-engine's `demos/*.json` with Imp Lab's seed tool, passing this configuration so notation documents become Notations records:

```bash
# from imp-lab/apps/tauri/src-tauri, with aura-engine at $E and aura-editor at $A
cargo run --example seed_project -- --extensions $A/pack/extensions.json \
  $E/lab-pack/demo-project "Aura demos" \
  $E/demos/demo-01.json $E/demos/demo-02.json $E/demos/demo-02-arpeggio.json $E/demos/sine.json
```
