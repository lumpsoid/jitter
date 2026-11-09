# Changelog

All notable changes to Jitter are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2026-11-09

### Added

- On a very wide window (2700 logical pixels or more), Top, Categories and
  Favorites sit side by side as three panes under one Home entry, instead
  of grids whose rows span the whole screen.
- SteamOS: how to get touch, not mouse, controls in Game Mode.

### Changed

- The chat beside the video takes a share of the window instead of a fixed
  width: narrower on small or scaled screens, wider on ultrawide ones.

## [0.1.2] - 2026-11-08

### Fixed

- The player keeps what it has loaded of a stream in the app's cache
  directory, instead of failing to and holding it all in memory.
- Android: the log no longer gives a screen it cannot measure an impossible
  size, and says which video output the player draws with, not the
  placeholder it starts on.

## [0.1.1] - 2026-11-08

### Added

- The log now names the device (OS and build, CPU, RAM, GPUs and their
  drivers, screens) and how the player decodes each stream, so playback bug
  reports can tell what went wrong. Nothing that identifies you is logged.

### Changed

- Windows builds play with a current mpv and FFmpeg.

### Fixed

- A VOD, or a live stream rewound into its recording, that stops loading
  (for example after a seek hits a dead connection) is reloaded after 15
  seconds where it was, instead of buffering until you leave. Live streams
  already recovered this way.
- Windows: seeking a VOD outside what has loaded no longer turns the
  picture black, and the recording of a stream that is still live can be
  rewound.

## [0.1.0] - 2026-11-06

The first release: Android, Linux (AppImage) and Windows.

### Added

- Browse the most-watched live streams, categories and a category's streams.
- Search channels and categories.
- Favorite channels, listed live first, kept on the device.
- Watch live streams and VODs at a chosen starting quality, resuming VODs
  where you left off.
- Rewind a live stream into its recording and go back to live.
- Live chat and VOD chat replay, with 7TV, BTTV and FFZ emotes, subs, raids
  and a chatter's messages on tap.
- Theatre mode with the chat beside or below, zoom and pan, a sleep timer,
  keyboard shortcuts, and seeking by double tap on phones.
- Audio-only playback in the background with a media notification.
- A local watch history.
- Light and dark themes, and settings for chat and the starting quality.
- A log per app run under Settings, to attach to bug reports.
- The version and commit under Settings and in `jitter --version`.
- A command-line head, `jitter`, on the same core.

[Unreleased]: https://github.com/lumpsoid/jitter/compare/v0.1.3...HEAD
[0.1.0]: https://github.com/lumpsoid/jitter/releases/tag/v0.1.0
[0.1.1]: https://github.com/lumpsoid/jitter/compare/v0.1.0...v0.1.1
[0.1.2]: https://github.com/lumpsoid/jitter/compare/v0.1.1...v0.1.2
[0.1.3]: https://github.com/lumpsoid/jitter/compare/v0.1.2...v0.1.3
