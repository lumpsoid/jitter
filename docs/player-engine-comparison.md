# Player engine load on Linux

Measured 2026-11-14 on one machine. This is a spike, not a decision.
fvp is a dev dependency used only by the bench. The app still ships
media_kit.

## Machine

- Arch Linux, kernel 7.2.11, Wayland session. No X11 runs.
- Intel Arrow Lake iGPU (i915) runs the desktop and Flutter's renderer
  (Impeller, OpenGL ES).
- NVIDIA RTX 5070 Ti Mobile, driver 615.78, idle at about 3 W.
- No `intel-media-driver`, so there is no VA-API on the iGPU. Neither
  engine can decode on the Intel GPU here.
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
  about 3 W. On a hybrid laptop without VA-API, this is the largest
  cost in the table. With `intel-media-driver` installed, fvp would
  try VA-API first and leave the dGPU idle. That was not measured.

## Gotchas found

- **Flutter 3.50 crashes mpv's Lua scripts.** `libflutter_linux_gtk.so`
  3.50 exports `_Unwind_*` symbols. When a Lua error unwinds in
  libmpv's script thread, LuaJIT resolves those symbols to Flutter's
  unwinder, and the process gets SIGSEGV at mpv start. This happened
  in 3 of 3 bench starts without the preload. Preloading
  `libgcc_s.so.1` puts the system unwinder first. The app built with
  Flutter 3.50 is likely affected the same way on its first stream.
  CI pins Flutter 3.47.8, which was not checked.
- **mpv's hwdec cannot be chosen on main.** `hwdecFor`
  (`lib/player/media_kit_engine.dart`) only handles gamescope. Otherwise
  media_kit sets `hwdec=auto`. The bench sets it itself with
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

## Recommendation

- fvp is worth a Windows spike. On Linux it composed into Flutter's
  texture at about a third of mpv's GPU cost, with less CPU and
  memory. The Windows trouble with ANGLE and D3D 11 interop sits in
  that composition layer, which fvp replaces with its own D3D 11 path.
  Run this bench on Windows before deciding. The Linux numbers do not
  transfer.
- For Android, prefer the official `video_player` (ExoPlayer, maintained
  by Google) over fvp. fvp would add libmdk where the platform player
  already exists. Both sit behind the same video_player API, so
  `PlayerEngine` can grow toward that API either way.
- Before any migration, `PlayerEngine` must cover what the player page
  uses today: positions, buffering, play/pause, volume, seek-by and
  frame step. The page also needs its own controls instead of
  media_kit's.
- Not measured: VA-API decoding on Intel (needs `intel-media-driver`),
  X11, battery drain over time, and latency to the live edge.
