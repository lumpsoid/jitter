# Contributing to Jitter

Thanks for wanting to help. Jitter is a small personal project, so a little
coordination up front saves everyone time.

## Start with an issue

For a feature, a behaviour change or a larger fix, I'd like you to
[open a proposal](https://github.com/lumpsoid/jitter/issues/new?template=proposal.yml)
first, so we can agree on the idea and where it goes before you write code.
Then open a pull request that links the issue (`Closes #123`).

Small, obvious fixes (a typo, a broken link, a one-line bug fix) can go
straight to a pull request.

Found a bug? [Report it](https://github.com/lumpsoid/jitter/issues/new?template=bug_report.yml)
with the version from **Settings → About** and, if you can, the log from
**Settings → Logs**.

## Setup

You need [Flutter](https://docs.flutter.dev/get-started/install),
[Rust](https://www.rust-lang.org/tools/install) and the rinf CLI at the same
version as the `rinf` crate:

```sh
cargo install rinf_cli --version 8.11.0 --locked
rinf gen          # the Dart bindings are generated, not checked in
make run          # the Linux app; or flutter run -d <device>
make cli ARGS="top"
```

On Linux you also need libmpv and GTK 3 headers (`libmpv-dev libgtk-3-dev`
on Debian and Ubuntu). The GTK head, `native/gtk`, needs GTK 4.14+ and
libadwaita 1.6+, so `cargo test` for the whole workspace needs those too.

### Android

The Android build uses libmpv jars rebuilt with two FFmpeg patches
([why](./third_party/media_kit_libs_android_video/libmpv/README.md)). They
are not in git. Download the ones the release builds use (needs the
[GitHub CLI](https://cli.github.com/)):

```sh
scripts/libmpv-android.sh fetch
```

## Where a change goes

Read [CLAUDE.md](./CLAUDE.md) and the [code principles](./docs/code-principles.md)
first. In short:

- Business logic and state go in `native/core`, tested with plain values.
- Talking to Twitch goes in `native/twitch`, which returns core types and
  makes no decisions.
- Storage goes in `native/store`.
- `native/hub` connects core to Flutter over rinf
  ([rules](./docs/rinf-bridge.md)); `lib/` only draws and runs platform
  effects. No business logic in Dart.
- If the CLI cannot do something without Flutter, logic leaked out of core.
- No `unwrap`, `expect` or `panic!` in Rust; return a `Result`.

## Before you push

```sh
make check
```

runs every gate CI runs (`scripts/check.sh`): the version check, `cargo
fmt --check`, `cargo clippy -D warnings`, `cargo test`, `rinf gen`, `flutter
analyze` and `flutter test`. `make fmt` formats both languages.

Add tests for what you change. Update the docs and add a line under
`## [Unreleased]` in [CHANGELOG.md](./CHANGELOG.md) for anything a user
would notice.

## Commits

One logical change per commit, in
[Conventional Commits](https://www.conventionalcommits.org/) form:
`type(scope): brief`, e.g. `feat(settings): show the version and commit`.
The body says why, and anything a reviewer needs that the diff does not
show.

| Type | When |
|---|---|
| `feat` | New code, or a change in behaviour |
| `fix` | A bug fix |
| `test` | Only tests |
| `docs` | Only documentation |
| `ci` | Only CI (`.github/workflows`) |
| `chore` | Everything else: build, scripts, dependencies, formatting, releases |

Name branches the same way: `feat/chat-search`, `fix/vod-seek`.

## License

By contributing, you agree that your contributions are licensed under the
project's [0BSD license](./LICENSE).
