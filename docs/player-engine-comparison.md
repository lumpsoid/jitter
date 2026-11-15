# Player engine load

Measured 2026-11-14 on two machines, a Linux laptop and an Android
phone. The measurements began as a spike. The Recommendation now
records the decisions taken so far.
fvp is a dev dependency used only by the bench. The app still ships
media_kit.

## Machine

- Arch Linux, kernel 7.2.11, Wayland session. The measured matrices
  have no X11 runs. The GUI bench checks under XWayland
  (`GDK_BACKEND=x11`) were functional runs, not measurements.
- Intel Arrow Lake iGPU (i915) runs the desktop and Flutter's renderer
  (Impeller, OpenGL ES).
- NVIDIA RTX 5070 Ti Mobile, driver 615.78, idle at about 3 W.
- In the first run there was no `intel-media-driver`, so no VA-API on
  the iGPU. Neither engine could decode on the Intel GPU. See
  [Linux with VA-API](#linux-with-va-api).
- Flutter 3.50. mpv from the system libmpv. fvp 0.40.0 with libmdk
  0.40.1.

## Method

- Same Flutter build, same window, and a bare picture with no controls
  for both engines.
- Stream: one Twitch VOD at 1080p60 (source), and one live channel at
  1080p60. Each run has a 15 s warm-up and a 60 s measured window.
- VOD rows have 3 runs each. Live rows have 1 run each.
- Columns come from `scripts/player-load.sh`:
  - CPU: the bench process's utime + stime, as % of one core.
  - RSS: average VmRSS.
  - iGPU render: the process's i915 render-engine time from DRM
    fdinfo, as % of wall time.
  - NVIDIA: device-level `nvidia-smi` utilization, decoder utilization
    and power draw. Nothing else used the NVIDIA GPU.
- Configurations:
  - **shipped**: mpv `hwdec=auto`, which picks `vulkan-copy` here. For
    fvp, the default decoder list: VA-API fails and CUDA is used.
  - **software**: mpv `hwdec=no`, fvp `FFmpeg`.
- A third configuration, mpv `nvdec` with fvp `CUDA`, is left out. mpv
  reported `hwdec-current=no` for every `nvdec` run, so those rows were
  software decoding again. fvp CUDA is the same as fvp shipped.

Reproduce with:

```sh
flutter build linux --profile -t bench/player_bench.dart
U=$(cargo run -q -p jitter_cli -- play --vod <id> --url 1080p60)
LD_PRELOAD=/usr/lib/libgcc_s.so.1 scripts/player-load.sh mpv "$U"
LD_PRELOAD=/usr/lib/libgcc_s.so.1 scripts/player-load.sh fvp "$U"
```

The bench reads these environment variables:

- `JITTER_BENCH_MPV_HWDEC`: the mpv `hwdec` to use.
- `JITTER_BENCH_FVP_DECODERS`: a comma-separated decoder list.
- `JITTER_BENCH_WARMUP` and `JITTER_BENCH_SECONDS`: the warm-up and
  measured window, in seconds.

`LD_PRELOAD` puts the system `libgcc_s` ahead of Flutter's unwinder.
Without it, mpv's Lua scripts crash. See the first item under
[Gotchas found](#gotchas-found).

### Windows

- `bench/bench_app.dart` is a GUI bench. The tester types channels, VOD
  ids or links. The Rust hub resolves them as the app does.
- One entry plays 3 times, because the D3D 11 device reset shows on the
  2nd stream.
- The tester picks one engine per launch (mpv or fvp) and presses Run
  test. Each stream gets a warm-up, then a measured window. The result
  screen has a Copy log button.
- Log counters:
  - CPU: the process's total processor time, from PowerShell
    `Get-Process`.
  - Memory: Dart's `ProcessInfo.currentRss`, sampled each second.
  - GPU: per engine type, from
    `typeperf "\GPU Engine(pid_<pid>_*)\Utilization Percentage"`. The
    values are summed per engine type and averaged over the samples.
  - mpv's hwdec, vo and drops.
  - The app's lifecycle states during the window (visible, focused).
  - For mpv, media_kit's own `VIDEO` lines. They carry the D3D 11
    device checks.
- fvp does not expose which decoder it opened. The `VideoDecode` GPU
  share shows whether it decodes in hardware.
- Built by `.github/workflows/windows.yml` with the `bench` checkbox.
  It is a profile build, because Flutter drops dev-dependency plugins
  (fvp) from release builds. The run publishes a prerelease
  `test-windows-bench-<sha7>`.
- Results: see [Windows results](#windows-results).

## Results

Medians. CPU is % of one core. iGPU is % of wall time.

| Config   | Source | Engine             | CPU % | RSS MB | iGPU render % | NVIDIA GPU % / dec % | NVIDIA W |
|----------|--------|--------------------|-------|--------|---------------|----------------------|----------|
| shipped  | VOD    | mpv (vulkan-copy)  | 76.5  | 563    | 47.5 ¹        | 5.6 / 8.8            | 4.3      |
| shipped  | VOD    | fvp (CUDA)         | 67.8  | 513    | 16.4          | 1.0 / 2.1            | 10.9     |
| shipped  | live   | mpv (vulkan-copy)  | 72.2  | 561    | 49.2          | 5.5 / 9.0            | 4.4      |
| shipped  | live   | fvp (CUDA)         | 72.4  | 519    | 17.3          | 1.0 / 2.0            | 10.9     |
| software | VOD    | mpv (no)           | 91.3  | 535    | 49.3          | 0 / 0                | 3.0      |
| software | VOD    | fvp (FFmpeg)       | 86.5  | 454    | 17.8          | 0 / 0                | 3.0      |
| software | live   | mpv (no)           | 92.6  | 513    | 51.6          | 0 / 0                | 2.9      |
| software | live   | fvp (FFmpeg)       | 95.6  | 458    | n/a ²         | 0 / 0                | 3.0      |

¹ Per run: 15.6 % (the run that dropped 2100 frames, CPU 48.6 %), no
reading (no i915 client at the window's edge), 47.5 %. The table shows
the one normal run.

² The only run read no i915 client at the window's edge.

Raw CPU per VOD run:

- shipped mpv: 48.6 (the 2100-drop run), 76.5, 79.2
- shipped fvp: 65.1, 67.8, 72.1
- software mpv: 91.3, 90.3, 96.0
- software fvp: 86.5, 84.7, 87.7

Dropped frames: mpv reports them (`frame-drop-count`). Normal runs
drop 0 to 32 per minute. Two outlier runs drop many more: the first
shipped mpv VOD run dropped 2100, and the first mpv `nvdec` run
(software in effect, not in the table) dropped 3037. fvp exposes no
drop counter, so smoothness is not compared.

Presented frame rate could not be measured either. On the Linux
embedder a video texture redraws without a Dart frame, so
`FrameTiming` never sees it.

## Reading

- **iGPU**: mpv's path costs about three times the render-engine time
  of fvp's (about 48 to 52 % against about 16 to 18 %), in every
  configuration. The cost is in composing the frame into Flutter's
  texture, not in decoding. Decoding ran on NVIDIA or on the CPU.
- **CPU**: with hardware decoding, fvp is lower on VOD (67.8 % against
  76.5 %) and about the same on live (72.4 % against 72.2 %). In
  software the two are about the same (86 to 96 % for both).
- **Memory**: fvp uses about 40 to 80 MB less.
- **NVIDIA power**: fvp's CUDA decoding keeps the dGPU at about
  10.9 W. mpv's `vulkan-copy` uses about 4.3 W, and the idle draw is
  about 3 W. Without VA-API, this is the largest cost in the first
  table. With `intel-media-driver` installed, both engines decode on the
  iGPU and the dGPU stays idle. See [Linux with VA-API](#linux-with-va-api).

## Linux with VA-API

The first run had no `intel-media-driver`. With it installed, both
engines decode on the Intel iGPU through VA-API. mpv's `hwdec=auto`
picks `vaapi`, and fvp's default decoder list picks `VAAPI`. The NVIDIA
GPU stays idle, at about 3 W, for both.

Runs: 3 VOD and 1 live per engine. Each run has a 15 s warm-up and a
60 s measured window. swayidle was paused and the monitors were on (see
the hidden-window gotcha below). Every run handed Flutter 60 frames/s.
A `LD_PRELOAD` shim counted calls to
`fl_texture_registrar_mark_texture_frame_available`. mpv dropped 0
frames.

Medians of the VOD runs (3) and the live run (1).

| Source | Engine      | CPU % | RSS MB | iGPU render % | iGPU video % |
|--------|-------------|-------|--------|---------------|--------------|
| VOD    | mpv (vaapi) | 66.9  | 435    | 44.5 ¹        | 15.4         |
| VOD    | fvp (VAAPI) | 64.8  | 315    | 15.9          | 15.4         |
| live   | mpv (vaapi) | 65.2  | 401    | 48.0          | 13.6         |
| live   | fvp (VAAPI) | 62.3  | 315    | 15.2          | 13.0         |

¹ Two of three mpv VOD runs read no i915 client at the window's edge.
The live run agrees.

Reading:

- **Decoding** costs the same on both engines: about 13 to 15 % of the
  video engine.
- **Composition** still differs. mpv's render engine time is about 44
  to 48 %, against about 16 % for fvp.
- **CPU** is within 2 to 3 points now. Most of fvp's earlier CPU lead
  came from mpv's `vulkan-copy`, which copies frames back to system
  memory.
- **Memory**: fvp uses about 120 MB less on VOD and about 90 MB less on
  live.
- Forced `vulkan-copy` and `CUDA` reruns reproduced the first table.
  fvp with CUDA still drew about 11 W on NVIDIA.

## Gotchas found

- **Flutter 3.50 crashes mpv's Lua scripts.** `libflutter_linux_gtk.so`
  3.50 exports `_Unwind_*` symbols. When a Lua error unwinds in
  libmpv's script thread, LuaJIT resolves those symbols to Flutter's
  unwinder, and the process gets SIGSEGV at mpv start. This happened
  in 3 of 6 bench starts without the preload. Preloading
  `libgcc_s.so.1` puts the system unwinder first. The app built with
  Flutter 3.50 is likely affected the same way on its first stream.
  CI pins Flutter 3.47.8, which was not checked.
- **`load-scripts=no` cannot be set through media_kit.** libmpv reads
  `load-scripts` only before `mpv_initialize`. media_kit 1.2.6 has no
  public hook for pre-init options. Its pre-init map holds only
  `vid=no`, plus the Android libass options. A local patch that added
  `'load-scripts': 'no'` to that map did not work. The patch was
  reverted, and nothing was committed. The bench crashed in 3 of 6
  starts both without and with the patch. gdb shows the SIGSEGV in
  thread `lua/stats`, inside `libluajit-5.1.so.2`, reached through
  `_Unwind_RaiseException` exported by `libflutter_linux_gtk.so`. The
  stack is the same with and without the patch. The patched run still
  started every built-in Lua script thread (`lua/stats`,
  `lua/console`, `lua/ytdl_hook`, `lua/select`). So the option never
  took effect: media_kit's initializer ignores the return value of
  `mpv_set_option_string`.
  **Verdict:** media_kit's configuration cannot reach it. The
  workaround is still `LD_PRELOAD=/usr/lib/libgcc_s.so.1`. A real fix
  needs either a Flutter without the exported `_Unwind_*` symbols, or a
  player plugin we own that sets `load-scripts=no` before init.
- **Native Wayland freezes the GUI bench.** With the local Flutter 3.50
  on niri (Wayland), `bench/bench_app.dart` sometimes freezes for good.
  The main thread is blocked in `gtk_widget_size_allocate` (Flutter's
  synchronous resize). The raster thread is blocked in Mesa EGL
  `wl_display_roundtrip_queue`. It happens with either engine, and with
  direct URLs too. Native Wayland: 0 of 3 hub runs finished. With
  `GDK_BACKEND=x11` (XWayland), every run finished: mpv 3 of 3, direct
  URLs 4 of 4, fvp 1 of 1. Local bench runs therefore use
  `GDK_BACKEND=x11`. The real app with Flutter 3.50 on Wayland is
  unchecked.
- **mpv's hwdec is set by `JITTER_HWDEC` on main.** `hwdecFor`
  (`lib/player/media_kit_engine.dart`) returns `JITTER_HWDEC` first when
  it is set. Otherwise it keeps the gamescope case, and media_kit sets
  `hwdec=auto`. The bench still sets hwdec itself with
  `JITTER_BENCH_MPV_HWDEC`.
- **mpv `nvdec` does not engage** under media_kit's render context
  here. mpv falls back to software decoding.
- **fvp crashed once at teardown.** It was a SIGSEGV after the
  measurement, in 1 of 12 fvp runs.
- **libmdk is a closed-source binary** from one maintainer. It is free
  for Flutter users, and fvp includes a key for it. fvp itself is
  BSD-3. libmdk is downloaded at build time.
- **Dev dependencies and local Flutter.** `flutter pub get` with a newer
  local Flutter rewrites `pubspec.lock` (`test_api`, `vector_math`) and
  the plugin registrants. Do not commit those. Flutter drops
  dev-dependency plugins only from release builds. So debug and profile
  builds of the app on every platform build fvp too.
- **A hidden window stops mpv's frames.** media_kit renders a frame only
  when Flutter draws. With the window off screen, mpv handed Flutter
  about 6 frames/s and dropped the rest, with any hwdec. fvp kept
  decoding and handed over 60 frames/s. So a hidden window costs almost
  nothing with mpv, and full decoding with fvp. Power is not measured.
  swayidle powered the monitors off after 600 s, and niri then throttled
  frame callbacks. Measure with the screen on. The Linux reruns paused
  swayidle (`kill -STOP`) and ran `niri msg action power-on-monitors`.
  The first Linux matrix ran without that wake lock. mpv's two outlier
  runs (2100 and 3037 drops) fit a monitor power-off. The other runs
  dropped few frames.

## Android

Measured on a OnePlus 6: Snapdragon 845, Adreno 630, 8 GB, Android 14,
1080×2280 at 60 Hz. Three engines:

- `mpv`: media_kit with the patched libmpv.
- `fvp`: fvp, with libmdk on `AMediaCodec`.
- `exo`: the official `video_player` on ExoPlayer. This is the same
  `FvpEngine` without `fvp.registerWith`.

All three decode with the hardware decoder `OMX.qcom.video.decoder.avc`.
mpv picks `hwdec=mediacodec-copy` and `vo=gpu`.

Method:

- Run `scripts/player-load-android.sh <engine> <url>` over adb. The
  bench is a profile build with the application id
  `io.github.lumpsoid.jitter.bench`. Profile builds get their own id, so
  they install next to a release build.
- Config reaches the app over `adb reverse tcp:8765`. Markers come back
  through logcat.
- The phone must stay unlocked. Runs behind the lock screen do not
  count. The first attempt ran behind it.
- Streams: the same Twitch VOD at 1080p60. Live: channel `caedrel` at
  1080p60 (H.264 High, 60 fps).
- Battery current and the Adreno GPU counters need root, so they are not
  measured.

Columns:

- App CPU: the app process's utime + stime, as % of one core.
- System CPU: all 8 cores busy, %, from `/proc/stat`. Hardware decoding
  runs in the media codec service, outside the app.
- PSS: from `dumpsys meminfo`, in MB.
- Frames shown/s: SurfaceFlinger timestats `averageFPS` of the app's
  layer. It counts updates of that layer, not decoded video frames, so
  it can read slightly above 60.

VOD rows are medians of 3 runs. Live `fvp` and `exo` are 1 run each.
Live `mpv` is 3 runs: the median, with the range in brackets.

| Source | Engine | App CPU % | System CPU % | PSS MB | Frames shown/s    | mpv drops/min        |
|--------|--------|-----------|--------------|--------|-------------------|----------------------|
| VOD    | mpv    | 155.4     | 53.1         | 271    | 54.7              | 431                  |
| VOD    | fvp    | 88.3      | 33.4         | 223    | 52.2              | –                    |
| VOD    | exo    | 95.2      | 32.8         | 216    | 61.4              | –                    |
| live   | mpv    | 56.8      | 37.8         | 247    | 10.2 (8.2-14.2)   | 2970 (2740-3069)     |
| live   | fvp    | 91.9      | 33.6         | 221    | 50.5              | –                    |
| live   | exo    | 95.5      | 33.4         | 181    | 61.5              | –                    |

Raw VOD per run:

- mpv app CPU 157.8, 155.0, 155.4; frames 54.9, 52.1, 54.7; drops 419,
  580, 431.
- fvp app CPU 88.0, 88.5, 88.3; frames 52.1, 52.2, 52.2.
- exo app CPU 95.4, 95.2, 95.2; frames 61.4, 61.4, 61.5.

Reading:

- **mpv is the heaviest on the whole system.** On VOD it uses about 53 %
  of all cores, against about 33 % for fvp and exo. It also uses about
  50 MB more memory. `mediacodec-copy` copies every decoded frame back
  to memory, and mpv's GPU output renders it again.
- **mpv falls apart on the live stream.** It showed 8 to 14 frames/s and
  dropped about 50 frames/s, in 3 of 3 runs. fvp and ExoPlayer were
  smooth on the same channel. mpv's CPU drops because it throws frames
  away. Why live and not VOD was not traced. Both streams are H.264 High
  1080p60.
- **ExoPlayer is the only engine that reached 60 frames/s on both VOD
  and live.** fvp showed about 50 to 52 frames/s on both.
- **fvp and ExoPlayer cost about the same**: app CPU 88 to 96 %, system
  about 33 %, PSS 181 to 223 MB.

## Windows results

- Machine: Windows 10 Pro, build 26200, 24 logical cores. The GPU
  model was not logged.
- Build 6ebbc3f, profile mode. Bench window 1707 × 996 at device pixel
  ratio 1.5.
- Target: live channel lunateac at 1080p60, Source quality. Each engine
  played 3 streams in a row, with one app launch per engine. Each stream
  had a 10 s warm-up and a 30 s measured window, with a 3 s gap between
  streams.
- CPU % is of one core, so it can go above 100.

| Engine | Stream | CPU % | RSS avg MB | RSS max MB | GPU 3D % | GPU VideoDecode % | Decoder | Drops           |
|--------|--------|-------|------------|------------|----------|-------------------|---------|-----------------|
| mpv    | 1      | 17.7  | 773        | 782        | 42.5     | 13.1              | d3d11va | 0 vo, 0 decoder |
| mpv    | 2      | 13.4  | 778        | 784        | 40.9     | 13.2              | d3d11va | 0 vo, 0 decoder |
| mpv    | 3      | 8.4   | 779        | 784        | 37.2     | 13.1              | d3d11va | 0 vo, 0 decoder |
| mpv    | mean   | 13.2  | 777        | –          | 40.2     | 13.1              | –       | –               |
| fvp    | 1      | 13.3  | 555        | 564        | 42.5     | 12.7              | MFT     | not exposed     |
| fvp    | 2      | 8.4   | 548        | 564        | 41.4     | 12.7              | MFT     | not exposed     |
| fvp    | 3      | 6.2   | 546        | 564        | 43.6     | 12.6              | MFT     | not exposed     |
| fvp    | mean   | 9.3   | 550        | –          | 42.5     | 12.7              | –       | –               |

Reading:

- **CPU and memory**: fvp used less CPU on every stream, 9.3 % against
  13.2 % on average. Its RSS was about 225 MB lower (550 MB against
  777 MB). The GPU load was the same: 3D 42.5 % against 40.2 %, and
  VideoDecode 12.7 % against 13.1 %.
- **Decoding**: both engines decode in hardware. mpv uses d3d11va. fvp
  opened the MFT (Media Foundation) video decoder on each stream. Its
  audio decoder is FFmpeg.
- **Drops**: mpv dropped no frames, at the video output or the decoder,
  on all three streams. Its video filter ran at 59.9 fps. fvp has no
  drop counter, so smoothness is not compared.
- **D3D 11 device reset**: no reset showed on this machine. With mpv,
  every device check at every step of the three streams logged "ANGLE's
  works, media_kit's works". The flush between streams (commit dab2ea0,
  see [windows-d3d11-device-reset.md](windows-d3d11-device-reset.md))
  ran each time. The fvp run had no errors and no device problems.

Caveats:

- CPU falls from stream to stream for both engines. The first stream
  includes start-up cost. Compare the rows per stream, not only the
  means.
- Both runs played in a visible window filling the screen, as the
  tester watched. The fvp run logged `visible=unknown` only because
  Flutter gave the bench no lifecycle state there.
- `position_s` (39 for mpv, 62 for fvp) is each engine's own live
  position. The two are not comparable.
- One machine and one channel.

## Recommendation

- **Windows: fvp is the primary candidate** (decided by the user,
  2026-11-14). media_kit is unmaintained, and fvp is actively
  maintained. So an fvp crash becomes an upstream report, not a fork we
  carry.
  - Accepted risk: libmdk is a closed-source binary from one
    maintainer.
  - Not deal-breakers: the one Linux teardown SIGSEGV (1 of 12 runs),
    and fvp decoding while the window is hidden. The app can turn video
    off when hidden, as it already goes audio-only in the background.
  - Windows bench, one tested machine: fvp used less CPU and memory
    than mpv at the same GPU load, and both decode in hardware. The mpv
    device reset did not show after the dab2ea0 flush, so it is not a
    deciding factor there. See [Windows results](#windows-results).
- Android: the data now backs ExoPlayer through the official
  `video_player`. It has the lowest memory, reached 60 frames/s on VOD
  and live, adds no closed-source library, and is maintained by Google.
  mpv is the worst of the three on this phone, especially on live. fvp
  is no better than ExoPlayer here. Both sit behind the same video_player
  API, so `PlayerEngine` can grow toward that API either way.
- **Linux: fvp too** (decided by the user, 2026-11-15). With VA-API,
  fvp's gain is the composition cost and memory, not CPU, so Linux alone
  would not justify a switch. But with Windows on fvp and Android on
  ExoPlayer, keeping mpv for Linux only would mean carrying media_kit,
  its forks and the Lua crash workaround for one platform. All desktops
  share one engine.
- **Don't rebuild media_kit as a general plugin.** If mpv ever comes
  back, own a minimal plugin sized to `PlayerEngine`. The
  graphics-interop layer is the upkeep that killed media_kit. Such a
  plugin could also set `load-scripts=no` before init, which media_kit
  cannot (see [Gotchas found](#gotchas-found)).
- **The local Rust HLS proxy idea is shelved.** Rewind already uses the
  recording VOD, and the stall reload is already driven from Rust.
  Revisit it only for rewind on channels without VODs.
- Before any migration, `PlayerEngine` must cover what the player page
  uses today: positions, buffering, play/pause, volume, seek-by and
  frame step. The page also needs its own controls instead of
  media_kit's.
- Not measured: X11 matrices, battery drain over time, latency to the
  live edge, and on Android the battery current and Adreno GPU counters
  (both need root).
- Dropped: the Android `hwdec=mediacodec` with `vo=mediacodec_embed`
  spike. Android goes to ExoPlayer, so mpv tuning there would not ship.
