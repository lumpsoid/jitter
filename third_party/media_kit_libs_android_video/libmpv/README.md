# libmpv for Android, patched

`media_kit_libs_android_video` 1.3.8, vendored. Upstream's `android/build.gradle`
downloads media-kit's [libmpv-android-video-build] v1.1.7 jars; this copy uses
the jars in `android/libs/`, which are that release rebuilt with one FFmpeg
patch. Everything else in the package is upstream's.

## Why

v1.1.7 bundles FFmpeg 6.0. Twitch serves many VODs as fragmented-MP4 HLS
(`init-0.mp4`, `N.mp4`). Seeking outside the buffered range in one makes
FFmpeg's MP4 demuxer reuse its stale fragment index after HLS restarts the
stream at offset 0, so it reads samples at wrong offsets
(`Invalid NAL unit size`), no frame decodes, and the player stays black.
Desktop is unaffected: it uses the system's newer FFmpeg.

`mov_reset_fragments.patch` is FFmpeg commit 380a518c43 ("avformat/mov: if
pos has been reset, clear fragments and indexes and search for next root",
[trac #7359]), first released in FFmpeg 8.0. It applies cleanly to 6.0.

How it was found, the logs, and the FFmpeg bisect are in
[docs/android-vod-seek-bughunt.md](../../../docs/android-vod-seek-bughunt.md).

## Rebuilding

`./build.sh [workdir]`, then paste the printed MD5s into
`android/build.gradle`. See the script header for prerequisites.

[libmpv-android-video-build]: https://github.com/media-kit/libmpv-android-video-build
[trac #7359]: https://trac.ffmpeg.org/ticket/7359
