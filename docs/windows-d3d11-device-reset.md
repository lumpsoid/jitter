# Workaround: flush mpv's video teardown between streams on Windows

**Status:** active since `7bce401` (2026-11-12), awaiting a tester's
confirmation on Windows. Remove it once the checks in
[Removing it](#removing-it) pass.

**Code:** `_flush` in `lib/player/media_kit_engine.dart`, and
`VideoOutput::Flush` in `third_party/media_kit_video/windows/video_output.cc`.

## Symptom

On Windows, the second live stream in a row plays sound with no picture. mpv
logs `Mapping hardware decoded surface failed.` for every frame. ANGLE's
D3D 11 device gets `DXGI_ERROR_DEVICE_RESET`, and on a third stream FFmpeg
fails with `0x887A0005` (`DXGI_ERROR_DEVICE_REMOVED`). Event Viewer has no
TDR entry, and media_kit's own D3D 11 device is never removed.

Reported on an NVIDIA RTX 3060 Ti, driver 32.0.16.1742, Windows 11 build 26200.
The result depends on `hwdec`:

| `hwdec`                | Three streams in a row |
|------------------------|------------------------|
| `d3d11va` (default)    | fails on stream 2      |
| `d3d11va-copy`         | plays                  |
| `no`                   | plays                  |

The `Testing FBO format` line appears for stream 1 and never for stream 2.
That line is logged by `check_gl_features`, which ends `init_video`, so
`init_video` does not run for stream 2.

## Cause

mpv's libmpv render path (`video/out/vo_libmpv.c`) applies a pending video
teardown only inside `mpv_render_context_render`. Going idle does not render,
so the teardown stays pending and is lost when the next stream starts. The
chain, with mpv at `48fca90e8`:

1. When playback stops, mpv goes idle and destroys the video output.
   `player/playloop.c:1046` `handle_force_window` calls `uninit_video_out`.
   Nothing sets `force-window`; media_kit sets `idle=yes`.
2. `video/out/vo_libmpv.c:685` `uninit()` does not free the GL state. It sets
   `ctx->img_params = {0}` and `need_reconfig = true`. Those are applied only
   in the next `mpv_render_context_render` (`vo_libmpv.c:371`).
3. media_kit renders only from mpv's update callback. That callback does not
   fire for a VO uninit, so nothing renders while mpv is idle.
4. The next stream calls `reconfig()` (`vo_libmpv.c:668`), which overwrites
   the pending `{0}` with params equal to stream 1's. `gl_video_config`
   (`video/out/gpu/video.c:4277`) sees no change, so `uninit_video` and
   `init_video` are skipped. The hwdec mapper and its EGLStream (created at
   `video/out/opengl/hwdec_d3d11egl.c:246`, destroyed at `:211`) are reused
   while still bound to stream 1's decoder texture array. FFmpeg creates a
   new decoder with a new 26-slice texture array on the same device, and the
   first post (`hwdec_d3d11egl.c:303`) resets the device.

Ruled out: ANGLE (`StreamProducerD3DTexture::postD3DTexture` only swaps the
texture, and each acquire rebuilds `TextureStorage11_External` and its SRV;
checked at ANGLE `2693b03eba82` and `97d96dab705c`), FFmpeg (`078390418`),
and media_kit. The symptom also reproduces on current ANGLE 2.1.29426 (tested
2026-11-10), so it is not an old-ANGLE bug.

Background: from November 2020 until mpv PR #18326 (merged 2026-07-31),
d3d11-egl could not initialize with any ANGLE. Follow-up #18605 is also
upstream. The zero-copy path is therefore only newly reachable for libmpv
render-API users such as media_kit.

## Workaround

Jitter flushes the video output at two points, and the flush renders once on
media_kit_video's worker thread:

- `MediaKitEngine._flush` calls the native `flush` in `stop()`, after the
  player stops, and in `open()`, before the next stream opens.
- One render with no video output applies the pending `{0}` reconfig. That
  frees the mapper, so the next stream initializes fresh.

The `open()` call is the one that matters: the next stream's reconfig waits on
the network, so mpv has gone idle by then. The flush is fire-and-forget. The
native call returns before the render, so the `stop()` flush can run before
mpv has torn the output down.

If the flush does not hold, the fallback is to default Windows to
`hwdec=d3d11va-copy`. That worked in the tester's run. It costs a GPU to CPU
to GPU copy of every frame.

## Removing it

The flush works around the mpv behavior in [Cause](#cause). Remove it once
either of these changes is in the libmpv that ships with Jitter:

1. `reconfig()` in `video/out/vo_libmpv.c` no longer lets a new reconfig
   overwrite a pending uninit reset, or `uninit()` now triggers the update
   callback.
2. `video/out/opengl/hwdec_d3d11egl.c` recreates the EGLStream when the
   source texture array changes.

Then a tester runs three live streams in a row on Windows with the flush
removed: the `_flush` calls in `stop()` and `open()`, the native `Flush`, and
its entry in `third_party/media_kit_video/PATCHED.md`. Three streams with a
picture on each means the workaround can go.

## References

- mpv PR #18326 (merged 2026-07-31):
  https://github.com/mpv-player/mpv/pull/18326
- mpv PR #18605 (follow-up): https://github.com/mpv-player/mpv/pull/18605
- mpv source at `48fca90e8`: `player/playloop.c`, `video/out/vo_libmpv.c`,
  `video/out/gpu/video.c`, `video/out/opengl/hwdec_d3d11egl.c` (paths in
  [Cause](#cause)).
- Jitter commits: `856b74a` (native flush), `7bce401` (Dart caller).
