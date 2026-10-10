# Input bindings design

Declarative requirements for configurable input→command bindings. Motivation: [`../scoping/product.md`](../scoping/product.md).

## Role

- The editor maps user input to **commands** through a configurable bindings layer.
- Bindings are the abstraction between input sources and application actions; UI chrome (buttons, menus) may invoke the same commands without going through bindings.
- Keyboard shortcuts are the first input source. Additional sources (for example mouse) may be added later using the same command and file model.

## Persistence

- Bindings load from an optional **user** JSON file, else an optional **bundled default** JSON file, else an empty bindings table.
- Load is **replace, not merge**: when the user file is present and valid, the default file is not consulted.
- The application must start and remain usable when neither file is present or when a file fails to load; missing bindings only means those actions are unavailable from input. Direct UI controls remain available.
- Unsupported schema versions are treated as a load failure for that file; the load chain continues.
- A bindings edit UI and writing the user file from the app are out of scope until designed. Users may hand-edit the user JSON file.

## Schema (version 1)

- Top-level fields: `version` (number) and `bindings` (array).
- Each binding has `command` (stable string id) and `when` (input trigger).
- Key triggers use `type` `key`, a `key` string (character or named key), and `modifiers` as a subset of `ctrl`, `alt`, `shift`, `logo`. The modifier set on the event must match exactly.
- Unknown command ids in a file are skipped (with logging) so files can outlive a given build.
- One trigger maps to at most one command; if duplicates appear, the last entry wins.
- Multiple bindings may target the same command.

## Commands

- Commands are identified by stable string ids and a known set in code.
- Initial commands: `document.open` (open graph file dialog) and `document.close_active` (close the active tab, or no-op when none).
- The editor does not expose or override native OS input commands (clipboard, quit, window manager, and similar). Defaults must not claim those shortcuts.

## Runtime

- Keyboard bindings resolve from ignored (uncaptured) key events so focused widgets can keep input later.
- Canvas pan and zoom remain widget-local until those actions are lifted to commands.

## Non-goals / avoided approaches

- Merging user and default binding tables entry-by-entry
- Requiring bindings files for the app to run
- Treating OS clipboard / window-manager shortcuts as editor commands
- A bindings settings UI in the current slice
- Persisting or migrating canvas camera controls through the bindings file yet

Background: [`../scoping/product.md`](../scoping/product.md).
