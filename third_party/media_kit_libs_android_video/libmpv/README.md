# libmpv for Android, patched

`media_kit_libs_android_video` 1.3.8, vendored. Upstream's `android/build.gradle`
downloads media-kit's [libmpv-android-video-build] v1.1.7 jars; this copy uses
the jars in `dist/libmpv-android/` at the repo root, which are that release
rebuilt with two FFmpeg patches. They are gitignored: build them once per
checkout. Everything else in the package is upstream's.

## Why

v1.1.7 bundles FFmpeg 6.0. Twitch serves many VODs as fragmented-MP4 HLS
(`init-0.mp4`, `N.mp4`). Seeking outside the buffered range in one makes
FFmpeg's MP4 demuxer reuse its stale fragment index after HLS restarts the
stream at offset 0, so it reads samples at wrong offsets
(`Invalid NAL unit size`), no frame decodes, and the player stays black.
Desktop is unaffected: it uses the system's newer FFmpeg.

`03-mov_reset_fragments.patch` is FFmpeg commit 380a518c43 ("avformat/mov: if
pos has been reset, clear fragments and indexes and search for next root",
[trac #7359]), first released in FFmpeg 8.0. It applies cleanly to 6.0.

How it was found, the logs, and the FFmpeg bisect are in
[docs/android-vod-seek-bughunt.md](../../../docs/android-vod-seek-bughunt.md).

A VOD that is still recording is an HLS EVENT playlist: all segments from
the start, but no `#EXT-X-ENDLIST`. FFmpeg 6.0 treats it as live: no
duration, a timeline anchored at the newest segments, and every seek drops
data up to the live edge, so it plays from the latest second and cannot
rewind.

`04-hls_seek_event_playlists.patch` is FFmpeg commit 6d98a9a2e8 ("avformat/hls:
fix seeking in EVENT playlists that start mid-stream", Kacper Michajłow),
first released in FFmpeg 9.0, found by bisecting 6.0 to 9.0. It needed one
context-only adjustment for 6.0 (noted in the patch) and must apply after
upstream's `hls_mp4_seek.patch`.

The number prefix is the apply order. Upstream's two FFmpeg patches are
unnumbered; `build.sh` renames them `01-dash_base_url_escape.patch` and
`02-hls_mp4_seek.patch` before the build, and fails if an unnumbered one
appears.

## Rebuilding

`./build.sh [workdir]` writes the jars to `dist/libmpv-android/`; paste the
printed MD5s into `android/build.gradle`. See the script header for
prerequisites.

[libmpv-android-video-build]: https://github.com/media-kit/libmpv-android-video-build
[trac #7359]: https://trac.ffmpeg.org/ticket/7359
