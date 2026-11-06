# Jitter

[![CI](https://github.com/lumpsoid/jitter/actions/workflows/ci.yml/badge.svg)](https://github.com/lumpsoid/jitter/actions/workflows/ci.yml)
[![License: 0BSD](https://img.shields.io/badge/license-0BSD-blue.svg)](./LICENSE)

Watch Twitch streams and VODs without an account. Jitter is a small player
for Android, Linux (including the Steam Deck) and Windows that keeps your
favorites and history on your device.

Jitter is not affiliated with, endorsed by, or sponsored by Twitch
Interactive, Inc. "Twitch" is a trademark of Twitch Interactive, Inc.

## Features

- **Browse** the most-watched live streams and categories, and **search**
  channels and categories.
- **Favorites** without an account, listed live first.
- **Live streams and VODs** at the quality you choose; VODs resume where you
  left off, and a live stream rewinds into its recording and back to live.
- **Chat** beside or below the video, live or replayed with a VOD, with 7TV,
  BTTV and FFZ emotes.
- **Background audio** with a media notification, and a sleep timer.
- **Zoom and pan** the picture, keyboard shortcuts on desktop, and gamepad
  and Steam keyboard support in Game Mode.
- A local **watch history**, light and dark themes, and per-run **logs** to
  attach to a bug report.

### Privacy

There is no login and no telemetry. Jitter talks to Twitch for streams and
chat, and to 7TV, BTTV and FFZ for emotes while "Third-party emotes" is on
in Settings. Favorites, history, resume points and settings stay in a
SQLite file on your device.

## Install

Download the file for your platform from the
[latest release](https://github.com/lumpsoid/jitter/releases/latest):

| Platform | File | |
|---|---|---|
| Android 7+ | `jitter-<version>-android.apk` | Allow installs from your browser or file manager when asked. |
| Linux x86-64 | `jitter-<version>-x86_64.AppImage` | `chmod +x` it and run it. Needs glibc 2.38+ (Ubuntu 24.04, Debian 13, Fedora, Arch, SteamOS 3). |
| Windows 10/11 x64 | `jitter-<version>-windows-x64.zip` | Unzip anywhere and run `jitter.exe`. It is not code-signed, so SmartScreen may warn the first time. |

Each release has a `SHA256SUMS` file. Check a download against it:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

On SteamOS, add the AppImage to Steam to play it in Game Mode; see
[Linux AppImage](./docs/linux-appimage.md).

The app shows its version and commit under **Settings → About**.

## Build from source

You need [Flutter](https://docs.flutter.dev/get-started/install) and
[Rust](https://www.rust-lang.org/tools/install). On Linux, also libmpv and
GTK 3 headers (`libmpv-dev libgtk-3-dev` on Debian and Ubuntu, `mpv gtk3` on
Arch).

```sh
git clone https://github.com/lumpsoid/jitter && cd jitter
cargo install rinf_cli --version 8.11.0 --locked
rinf gen
flutter run --release
```

`rinf gen` writes the Dart side of the Rust bridge, which is not checked in.
Android builds also need a patched libmpv; see
[CONTRIBUTING](./CONTRIBUTING.md#android).

There is a command-line head too: `cargo run -p jitter_cli -- --help`.

## How it is built

All the logic lives in Rust; Flutter draws the screens and plays the video
through [media_kit](https://github.com/media-kit/media-kit) (libmpv). The
two talk over [rinf](https://github.com/cunarist/rinf).

```
native/core    business logic and state; no I/O, tested with plain values
native/twitch  Twitch access (GQL, playback, chat); returns core types
native/store   SQLite storage for core
native/hub     the Rust side of the Flutter app
native/cli     the command-line head
native/gtk     an experimental native GTK head
lib/           the Flutter app
```

More in [CLAUDE.md](./CLAUDE.md), [code principles](./docs/code-principles.md)
and [the rinf bridge](./docs/rinf-bridge.md).

## Contributing

Bug reports and ideas are welcome. Please read
[CONTRIBUTING.md](./CONTRIBUTING.md) first: for anything bigger than a small
fix, open an issue to talk it through before a pull request.

## License

Jitter's own code is under the [0BSD license](./LICENSE). It does not cover:

- `third_party/`: vendored, patched copies of media_kit packages, each under
  its own `LICENSE` (MIT). The exception is the `*.patch` files in
  [`third_party/media_kit_libs_android_video/libmpv/`](./third_party/media_kit_libs_android_video/libmpv/):
  those are upstream FFmpeg commits backported to FFmpeg 6.0. They keep their
  original authors and stay under FFmpeg's license.
- libmpv and FFmpeg. They are not in this repository, but release builds
  bundle them: the AppImage carries Ubuntu 24.04's packages, the Android
  build carries media-kit's libmpv-android-video-build rebuilt with the
  patches above by that directory's `build.sh`, and the Windows build carries
  media-kit's libmpv-win32-video-build. They stay under their own licenses
  (LGPL or GPL, depending on the build), and their source is available from
  their upstream projects, plus those patches.
