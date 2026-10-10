# Testing design

Declarative testing requirements for Aura Editor. Product context: [`product.md`](product.md).

## Goals

- The editor is heavily test-driven and designed so behavior is easy to test.
- Both **unit** and **functional** tests are required.
- Prefer aggressive coverage of anything that can be checked consistently and repeatedly.

## What to test

- Pure application state and `update` logic (unit tests).
- Bindings JSON parse/load chain, trigger resolve, and command dispatch (unit tests; do not depend on the real user config directory).
- Stable headless UI interactions via Iced’s testing support (`iced_test` Simulator and similar): basic clicks, key events, and presence of labeled controls/text when selectors are reliable.
- Invariants that do not depend on timing, pixel layout, or audio output.

## What not to test

- Cases prone to race conditions or flaky timing.
- Brittle, hacky mock or fake integrations that are hard to keep honest.
- Deep analysis of visual output (pixel snapshots, layout geometry beyond simple widget presence) or audio output.
- Input sequences that are difficult to generate and consume reliably.

## Structure

- Keep a thin binary entrypoint; put application model, messages, update, and view in a library (or equivalent) so tests call them directly.
- Functional tests must run headless and must not require a real display server.
