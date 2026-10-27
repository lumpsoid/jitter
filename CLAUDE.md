# Jitter

A GUI application for watching Twitch streams.

@docs/code-principles.md

## Stack

- **Rust** — all business logic, state, and Twitch access.
- **Flutter** — GUI head and platform effects only. No business logic in Dart.
- **[rinf](https://github.com/cunarist/rinf)** — the Rust↔Flutter bridge.
  Layering and signal rules: [rinf bridge](./docs/rinf-bridge.md).

## Layout

```
native/core   (jitter_core)    business logic + state. No rinf, no tokio, no I/O.
native/twitch (jitter_twitch)  Twitch access; returns core types.
native/store  (jitter_store)   SQLite; implements core's storage ports.
native/hub    (hub)            rinf bridge — the Flutter head's Rust side.
native/cli    (jitter_cli)     CLI head.
lib/                           Flutter GUI head.
```

Each interaction head (GUI, TUI, CLI) is its own crate on top of core,
twitch and store. Dependencies point from heads to twitch and store, and
from those to core, never back.

## Architectural commitments

1. Rust owns state, in feature controllers that each own one state object.
   Rust sends view state to Flutter as `RustSignal`s; Dart sends user intents
   as `DartSignal`s.
2. The core is usable without Flutter: exercised by `cargo test` and the CLI.
   If the CLI cannot do something without Flutter, logic leaked out of core.
3. Core does not know Twitch's wire formats; twitch does not decide.
4. No panics in Rust (`unwrap`/`expect`/`panic!`); return `Result`.

## Gates

`make check` runs all of them in order. Individually:

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`
- `rinf gen` after changing any signal struct, and on a fresh checkout
  before `flutter analyze` (the generated bindings are gitignored)
- `flutter analyze` / `flutter test` for the GUI
