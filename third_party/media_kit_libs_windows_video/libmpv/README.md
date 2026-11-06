# libmpv for Windows, from shinchiro's builds

`media_kit_libs_windows_video` 1.0.11, vendored without `example/`.
Upstream's `windows/CMakeLists.txt` downloads media-kit's
[libmpv-win32-video-build] 2023-09-24 archive; this copy downloads the
archive pinned in `archive.md5` from jitter's GitHub release
`libmpv-windows-<first 8 of its MD5>` instead. Everything else in the
package, ANGLE included, is upstream's.

The archive is an unchanged `mpv-dev-x86_64-*.7z` from [shinchiro's
mpv-winbuild-cmake], mpv and FFmpeg built from git, taken from its
[SourceForge mirror]. `scripts/libmpv-windows.sh update` checks it against
the MD5 SourceForge publishes.

## Why

media-kit's Windows build bundles FFmpeg 6.0 and was archived in October
2024. FFmpeg 6.0 has the two VOD seek bugs the Android build patches (see
[its README](../../media_kit_libs_android_video/libmpv/README.md)): seeking
outside the buffer in a fragmented-MP4 VOD goes black
(`Invalid NAL unit size`), and a VOD that is still recording cannot be
rewound. Both fixes are upstream since FFmpeg 9.0, so a current build needs
no patches. media-kit's other builders are still on FFmpeg 6.0 too, so
updating media_kit would not help.

shinchiro publishes a build daily on GitHub but keeps those releases only
briefly; SourceForge gets about one a week and keeps them. Jitter keeps its
own copy so an old app version always builds.

## Updating

`scripts/libmpv-windows.sh update [date]`, then `publish`, then commit
`archive.md5`. See [maintaining](../../../docs/maintaining.md#libmpv-for-windows).
After a local update, delete `build/windows/`: CMake does not re-extract
an archive over an already extracted one.

## License

The archive is GPLv3: FFmpeg is configured with `--enable-gpl
--enable-version3`. Its source is [shinchiro's mpv-winbuild-cmake] and the
upstream [mpv] (the git revision in the file name) and [FFmpeg] it builds.

[libmpv-win32-video-build]: https://github.com/media-kit/libmpv-win32-video-build
[shinchiro's mpv-winbuild-cmake]: https://github.com/shinchiro/mpv-winbuild-cmake
[SourceForge mirror]: https://sourceforge.net/projects/mpv-player-windows/files/libmpv/
[mpv]: https://github.com/mpv-player/mpv
[FFmpeg]: https://github.com/FFmpeg/FFmpeg
