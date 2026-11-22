# Changelog

All notable changes to Jitter are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-11-22

### Added

- Watch a downloaded VOD in the app: tap it under Settings → Downloads.
  A VOD you have downloaded plays from its file wherever you open it,
  offline too, and carries on from where you stopped.
- Finished downloads stay listed after the app restarts. Removing one from
  the list keeps its file.

### Changed

- On Android 11 and later, downloads are saved to `Download/Jitter`, where
  any file manager can reach them, instead of the app's own folder.

### Fixed

- A downloaded fMP4 VOD's timestamps start at 0:00, so other players show
  its time from the start of the file.
- "Download started" shows only once the download has really started; a
  download that fails to start says why instead.
- A failure for one VOD's download no longer shows on another's.
- Opening the app while a command-line `jitter` holds the database waits
  for it instead of failing.

## [0.2.0] - 2026-11-21

### Added

- Download a VOD: long-press its tile on a channel page, pick a quality,
  and follow it, cancel it or remove it under Settings → Downloads. From
  the command line, `jitter download` saves a VOD to one file. A download
  fetches four segments at once, and one that was interrupted resumes where
  it stopped instead of starting over.
- Setting the environment variable `JITTER_ENGINE=fvp` plays video with
  fvp instead of media_kit on Linux and Windows, to compare the two.
- Export your favorites, watch history, VOD positions and settings to a
  JSON file, and import one, from Settings → Your data or with
  `jitter export` and `jitter import`. An import adds to what you have and
  removes nothing: a favorite or visit you already have is kept, a VOD
  position is replaced only by a newer one, and the file's settings are
  taken. On Android and iOS the export goes to the share sheet.
- Setting the environment variable `JITTER_HWDEC` makes the player decode
  video with that mpv `hwdec`, for example `d3d11va-copy`, or `no` for
  software decoding, to try another decoder when a GPU driver misbehaves.

### Changed

- The log keeps what mpv's video decoder and renderer report, and on
  Windows what the video output does and the first time a GPU device it
  draws with stops working, with Windows' reason. On Windows it also
  records the GPU devices' state as each stream stops, opens, loads and
  starts decoding.
- The app shows a still "Loading…" mark while it starts, instead of
  spinners.

### Fixed

- On Linux, the video no longer freezes for a moment each time the player
  controls fade in or out.
- The play/pause button no longer flips from play to pause each time the
  controls show over a stream that is already playing.
- On Windows, the second stream in a row shows its picture again, instead of
  playing sound over a black video with zero-copy hardware decoding.

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

[Unreleased]: https://github.com/lumpsoid/jitter/compare/v0.2.1...HEAD
[0.1.0]: https://github.com/lumpsoid/jitter/releases/tag/v0.1.0
[0.1.1]: https://github.com/lumpsoid/jitter/compare/v0.1.0...v0.1.1
[0.1.2]: https://github.com/lumpsoid/jitter/compare/v0.1.1...v0.1.2
[0.1.3]: https://github.com/lumpsoid/jitter/compare/v0.1.2...v0.1.3
[0.2.0]: https://github.com/lumpsoid/jitter/compare/v0.1.3...v0.2.0
[0.2.1]: https://github.com/lumpsoid/jitter/compare/v0.2.0...v0.2.1
