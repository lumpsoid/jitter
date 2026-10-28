# Android: VODs go black after a seek

A case record: what broke, how it was found, how the FFmpeg fix was
bisected, and what the repo carries now. The short version lives in
[`third_party/media_kit_libs_android_video/libmpv/README.md`](../third_party/media_kit_libs_android_video/libmpv/README.md).

## Symptom

On Android only:

- Opening a VOD from a resume point loaded for a long time, then showed a
  black frame and never went on. Chat loaded and replayed fine.
- Starting from 0 played. A seek (double tap, seek bar) **into time already
  buffered** worked; a seek **outside the buffer, in either direction**
  loaded for a long time and went black for good.
- Switching quality (even to 160p30, to rule out the network) hid the
  spinner and left the black frame. No later seek recovered it.
- Desktop (Linux) played the same VODs, resume and seeks included.

## Investigation

Each step below names the hypothesis, what tested it, and the verdict. The
wrong turns are kept: they are what the logs ruled out.

### 1. Android surface reset racing the start seek — wrong

`MediaKitEngine.open` passes the resume point as `Media(url, start:)`;
media_kit sets mpv's `start` in its `on_load` hook. On Android only,
media_kit_video's `AndroidVideoController` (`widListener`) answers the first
`videoParams` by resizing the surface, re-creating the video output
(`vo=null` → `wid` → `vo=gpu`) and seeking to `player.state.position`. The
guess was that this seek collided with mpv's own start seek.

Test: when stuck on black, seek again. If the decoder only lacked feeding
after the reset, the picture would return. **It stayed black**, and a seek
from a VOD started at 0 failed the same way, so the start position was not
the cause.

### 2. Logcat only — decoder flushes, nothing out

The first log (`adb logcat`, no mpv output) showed each open, seek and
quality switch flushing or re-allocating `c2.exynos.h264.decoder` in
ByteBuffer mode (mpv's `hwdec=auto-safe` → `mediacodec-copy`):

```
D CCodec  : allocate(c2.exynos.h264.decoder)
D CCodec  : Client requested ByteBuffer mode decoder w/o color format set
D CCodecBufferChannel: [c2.exynos.h264.decoder#872] MediaCodec discarded an unknown buffer
D AudioTrack: stop(3198): called with 0 frames delivered
```

Audio barely played either, so the whole pipeline stalled, not just the
picture. Two suspects remained: the hardware decoder wedging after a
flush, or a network hang in the old FFmpeg (keep-alive reuse after a
mid-segment seek, seen as `keepalive request failed … retrying` in a
desktop run). Logcat could not tell them apart.

### 3. mpv verbose log — corrupt H.264 after the seek

A temporary change (not committed) turned on mpv's log and printed it:

```dart
Player(configuration: const PlayerConfiguration(logLevel: MPVLogLevel.v))
  ..stream.log.listen((l) => debugPrint('mpv[${l.prefix}] ${l.text}'));
```

The log is huge: `hls: Skip ('#EXT-X-PROGRAM-DATE-TIME:…')` repeats once per
segment (3481 of 3637 mpv lines in one capture). Filter it first, e.g.:

```sh
grep 'mpv\[' log.txt | grep -v 'EXT-X-PROGRAM-DATE-TIME'
grep -o 'mpv\[[^]]*\] .\{0,80\}' log.txt | sed -E 's/[0-9]+/N/g' | sort | uniq -c | sort -rn
```

Also note that the timestamps are when Dart printed the line, not when
mpv logged it; lines arrive in bursts with the same timestamp.

What it showed, for a resume at 14915 s of a 1080p60 VOD:

```
mpv[ffmpeg/demuxer] hls: Opening '1080p60/init-0.mp4' for reading
mpv[ffmpeg/demuxer] hls: Opening '1080p60/0.mp4' for reading
mpv[vd] Trying hardware decoding via h264_mediacodec-mediacodec-copy.
mpv[lavf] execute seek (to 14977.016333 flags 32)
mpv[lavf] seek done
mpv[cplayer] hr-seek, skipping to 14915.000000 (no framedrop)
mpv[ffmpeg/demuxer] hls: Opening '1080p60/init-0.mp4' for reading
mpv[ffmpeg/demuxer] hls: Opening '1080p60/1491.mp4' for reading
mpv[ffmpeg] NULL: Invalid NAL unit size (2140377624 > 15686).
mpv[ffmpeg] NULL: missing picture in access unit with size 15690
mpv[lavf] queuing seek to 14987.016333
mpv[cplayer] hr-seek, skipping to 14925.000000 (no framedrop)
mpv[lavf] queuing seek to 14997.016333 (cascade)
mpv[ffmpeg] https: Opening '1080p60/1493.mp4' for reading
… 1494.mp4 … 1545.mp4
```

Read it as:

- This VOD is **fragmented-MP4 HLS** (`init-0.mp4`, `N.mp4`). The VOD tested
  on desktop in step 2 was `.ts`; Twitch serves both.
- The network was fine: the seek completed and segments kept downloading.
- The bytes read after the seek were garbage. `Invalid NAL unit size` with
  impossible sizes means samples were cut at wrong offsets.
- No frame decoded, so mpv's hr-seek kept chasing forward (the `cascade`
  lines) and downloading. That is the "long load into black".
- Buffered seeks log `...using this range for in-cache seek.`: mpv serves
  them from its own cache, and FFmpeg never seeks. That is why they worked.

### 4. Software decoding — still corrupt, so it is the demuxer

The `NULL:` context might have meant the `h264_mp4toannexb` filter on the
MediaCodec path. A second temporary change forced software decoding
(`VideoControllerConfiguration(hwdec: 'no')`). The third log showed
`mpv[vd] Using software decoding.` and, after the first uncached seek, the
same error:

```
mpv[lavf] execute seek (to 81.782333 flags 32)
mpv[ffmpeg] NULL: Invalid NAL unit size (-1329290485 > 10052).
mpv[ffmpeg] NULL: missing picture in access unit with size 10056
```

So the decoder is cleared. The fault is in FFmpeg's demuxing (HLS feeding
the MP4 demuxer) before any decoder sees the data.

### 5. Versions

- The Android libmpv comes from `media_kit_libs_android_video` 1.3.8, which
  downloads media-kit's
  [libmpv-android-video-build](https://github.com/media-kit/libmpv-android-video-build)
  **v1.1.7** jars. `strings libmpv.so` shows `n6.0` (FFmpeg 6.0), with
  mbedtls.
- That repo's latest release (v1.1.11) still pins `v_ffmpeg=6.0`. It already
  carries `patches/ffmpeg/hls_mp4_seek.patch`
  ([media-kit#395](https://github.com/media-kit/media-kit/issues/395)), which
  clears `cur_init_section` on seek so the init segment is fetched again.
  The logs show that happening (`init-0.mp4` reopened after the seek), and
  the data is still corrupt, so that patch is not enough.
- Desktop mpv uses the system FFmpeg (libavformat 63, FFmpeg 8.x). Desktop
  mpv, same VOD, same 14915 s start, decoded fine.

## Reproducing on desktop

The phone round trip was the slow part, so the bug was reproduced with the
FFmpeg command-line tool on desktop. An `ffmpeg -ss` before `-i` seeks
through `hls_read_seek`, as mpv does for an uncached seek.

Get a fragmented-MP4 VOD's playlist with the CLI head (VOD `2912065706`,
channel pgl, at the time of writing):

```sh
cargo run -q -p jitter_cli -- play --vod 2912065706 --url 160p30
```

Build a minimal FFmpeg at a tag. `--disable-asm` is needed because 6.x
inline asm fails to assemble with current binutils (`operand type mismatch
for 'shr'`). From 8.0 on, the `tls` code needs `udp` enabled to link.

```sh
./configure --disable-everything --disable-doc --disable-asm --disable-autodetect \
  --enable-openssl --enable-network \
  --enable-demuxer=hls,mov,mpegts,aac --enable-decoder=h264,aac --enable-parser=h264,aac \
  --enable-protocol=http,https,tcp,tls,file,crypto,hls,data,udp \
  --enable-muxer=null --enable-encoder=wrapped_avframe,pcm_s16le \
  --enable-filter=null,anull,aresample,format,aformat,scale \
  --enable-bsf=h264_mp4toannexb --enable-swscale --enable-swresample
make -j"$(nproc)" ffmpeg
```

The test: seek, decode 3 s of video, count frames. 160p30 gives 90 frames
when it works.

```sh
ffmpeg -hide_banner -nostdin -ss 14915 -i "$URL" -t 3 -map 0:v:0 -f null - 2>&1 \
  | grep -E 'Invalid NAL unit size|missing picture|frame='
```

Results (all builds with media-kit's `hls_mp4_seek.patch` unless noted):

| FFmpeg              | Result                                                |
|---------------------|-------------------------------------------------------|
| n6.0                | `Invalid NAL unit size (-927609498 > 785)`, 0 frames  |
| n6.0, no patch      | `Invalid NAL unit size (986868072 > 785)`, 0 frames   |
| n6.1                | broken, 0 frames                                      |
| n7.0                | broken, 0 frames                                      |
| n7.1                | broken, 0 frames                                      |
| n8.0 (patch does not apply; that code was rewritten) | 90 frames |
| system (8.x)        | 90 frames                                             |

This reproduces the phone's exact error with no phone involved. It also
shows that no FFmpeg release before 8.0 has the fix.

## Bisecting FFmpeg

`n7.1` sits on a release branch, so the bisect ran from its branch point
on master, `e1094ac45d` (`*/version.h: bump minor versions for
release/7.1`, 2024-09-24), to `n8.0`. That is 3521 commits; limiting it to
`libavformat` and `libavcodec` cut it to 12 steps.

```sh
git clone --shallow-since=2024-08-01 https://github.com/FFmpeg/FFmpeg.git src-bisect
cd src-bisect
git bisect start --term-old=broken --term-new=fixed n8.0 e1094ac45d -- libavformat libavcodec
git bisect run ../bisect-step.sh
```

`bisect-step.sh` configured and built as above (exit 125, meaning skip, if
the build failed), ran the seek test, and returned 0 ("broken") or 1
("fixed", 90 frames). Steps, in order:

```
638690b872 avcodec: remove deprecated FF_API_VDPAU_ALLOC_GET_SET            fixed
9a794897c3 avcodec/sanm: simplify codec37 subcodec 3/4 path                 fixed
e2a8ece352 avformat/flvdec: support all multi-track modes                   fixed
61ff3047c5 avcodec/rv60dec: Check NEXT/LAST availability                    fixed
86460a0342 lavf/flvdec: replace a private option with a field in FFFormatContext  broken
d724584d68 avcodec/nvenc: set bitstreamRestrictionFlag when neccesary       broken
c8c6319ce1 avcodec/rv60: prevent decode_cu_r deadlock                       broken
34c06f5284 avcodec/escape130: move get_buffer down                          fixed
7f51cf75c6 avformat/hls: improve comment                                    fixed
13129f1af4 lavc: deprecate the omx encoders                                 broken
2f34b159b4 avcodec/eatgq: decode motion vector macroblocks                  broken
380a518c43 avformat/mov: if pos has been reset, clear fragments and indexes and search for next root  fixed
```

First fixed commit:

```
commit 380a518c439d4e5e3cf17b97e4a06259e8048f99
Author: vectronic
Date:   Mon Nov 4 14:07:41 2024 +0000

    avformat/mov: if pos has been reset, clear fragments and indexes and search for next root

    fixes https://trac.ffmpeg.org/ticket/7359

 libavformat/mov.c | 39 ++++++++++++++++++++++++++++++++++++---
```

This matches the logs. When HLS seeks, it re-opens the segment, so the MP4
demuxer's I/O position jumps back to 0. The demuxer still holds the
fragment index (`frag_index`) built from the earlier position, so it
resolves samples against stale fragments and reads at wrong offsets. The
commit makes `mov` notice the reset, drop the fragments and indexes, and
search for the next top-level box.

### Backport check

`git format-patch -1 380a518c43` applies to `n6.0` with offsets only (hunks
at −1930 and −1989 lines). n6.0 with media-kit's patch plus this one:

| VOD                         | Seek to  | Result     |
|-----------------------------|----------|------------|
| 2912065706 (fMP4)           | 14915 s  | 90 frames  |
| 2912065706 (fMP4)           | 60 s     | 90 frames  |
| 2912065706 (fMP4)           | 30000 s  | 90 frames  |
| 2911148898 (fMP4)           | 1800 s   | 90 frames  |
| 2912343811 (`.ts`)          | 1800 s   | 90 frames  |

## The Android build

Built on the Arch host, not a container:

- the upstream scripts only call `apt-get` or `yum` when those exist;
- `nasm` was installed for the x86 and x86_64 FFmpeg and dav1d builds;
- NDK `25.2.9519653`, the one the scripts pin, was installed next to the
  existing NDK. The scripts also install `build-tools;33.0.2`,
  `platforms;android-33` and `cmake;3.22.1`.

Starting from libmpv-android-video-build at tag **v1.1.7** (the release the
app already shipped), the changes were:

1. add `mov_reset_fragments.patch` to `buildscripts/patches/ffmpeg/`.
   `patch.sh` applies it after upstream's `dash_base_url_escape.patch` and
   `hls_mp4_seek.patch`;
2. drop `sudo` from `bundle_default.sh` and `build.sh`;
3. point the strip step at NDK 25.2. v1.1.7's `bundle_default.sh` hard-codes
   `ndk/27.3.13750724/…/llvm-strip`, the NDK its CI runner happened to
   have; the libraries themselves compile with the pinned 25.2 (r25c).

Build log notes that look like errors but are not:

- `rm: cannot remove 'deps'` / `'scripts/ffmpeg.sh'`: the script's own
  cleanup, whose guards are inverted (`if [ ! -f … ]`).
- libxml2's `configure: error: cannot run C compiled programs`: a probe
  while cross-compiling. The build goes on.
- FFmpeg's `…-android-nm: command not found`: a probe for a tool name newer
  NDKs dropped. The script passes `--nm=llvm-nm`.

The four ABIs built in about 7 minutes on 24 cores. Each jar holds
`libmpv.so` and `libmediakitandroidhelper.so`, at sizes close to upstream's
(arm64 is 5,730,463 bytes; upstream v1.1.11's is 5,730,435).

## What the repo carries

- `third_party/media_kit_libs_android_video/` is the 1.3.8 plugin,
  vendored. Its `android/build.gradle` differs from upstream's in one
  respect: instead of downloading jars it uses `android/libs/default-*.jar`,
  and a `verifyLibmpvJars` task (run before `preBuild`) fails the build if
  a jar is missing or its MD5 does not match:

  | Jar                       | MD5                                |
  |---------------------------|------------------------------------|
  | `default-arm64-v8a.jar`   | `6c12e38dfa56756c310e37f92b414a11` |
  | `default-armeabi-v7a.jar` | `3fafc94e47ee3a018b0f7e185915ece7` |
  | `default-x86_64.jar`      | `3ea724c13852964537b8e82129a861f4` |
  | `default-x86.jar`         | `ff6eb6c7f9d15ba4a35da3f3adaab9e3` |

- `third_party/media_kit_libs_android_video/libmpv/` holds
  `03-mov_reset_fragments.patch`, `04-hls_seek_event_playlists.patch` (a
  later fix for VODs that are still recording; see that folder's
  `README.md`), and `build.sh`, which reproduces the build above and prints
  the MD5s to pin.
- `pubspec.yaml` points `dependency_overrides` at the vendored plugin.
- `analysis_options.yaml` excludes `third_party/**`: the analyzer linted
  the vendored `pubspec.yaml` (`sort_pub_dependencies`).

There are no app-logic changes. `MediaKitEngine` is as it was, and the
temporary logging and software-decoding changes were reverted.

Verified: `make check` passes, and a debug arm64 APK's
`lib/arm64-v8a/libmpv.so` is byte-identical to the one in our jar
(`bdde135f…`) and differs from upstream v1.1.7's (`1400e6fe…`).
Not yet verified at the time of writing: playback on the phone. That means
resume, uncached seeks in both directions, and quality switches on a
fragmented-MP4 VOD.

## Maintenance

- **Rebuilding:** `third_party/media_kit_libs_android_video/libmpv/build.sh
  [workdir]`, then paste the printed MD5s into `android/build.gradle`.
- **Upgrading media_kit:** if `media_kit_libs_android_video` moves past
  1.3.8, re-vendor it and redo the `build.gradle` change. If it pins a new
  libmpv-android-video-build release, rebuild from that tag with the patch.
- **Retiring this:** once the upstream Android build ships FFmpeg 9.0 or
  later (8.0 has the mov fix but not the EVENT-playlist one), or carries
  both patches, delete `third_party/`, the override and the
  analyzer exclude. Check with the desktop seek test above against the new
  jar's FFmpeg version before trusting it.
- **If seeks break again:** capture an mpv verbose log as in step 3, filter
  the `EXT-X-PROGRAM-DATE-TIME` lines, and look at what follows the first
  `execute seek`.
