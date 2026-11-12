# media_kit_video, patched

`media_kit_video` 2.0.1, vendored without `example/`. Three patches differ
from upstream: hardware rendering on X11 (`linux/video_output.cc`),
`Video.pictureBuilder`, and the Windows video output's log (both below).

## Why

On an X11 session, upstream renders video in software: the log shows
`EGL display or context is invalid.` then `S/W rendering.`, and a 1080p
stream on a Steam Deck in Game Mode (gamescope's XWayland) plays at about
15 fps.

Upstream takes the EGL display and config for its isolated mpv context from
whatever EGL context is current on the GTK thread when the video output is
created. That is GDK's, not Flutter's: Flutter's view makes GDK's GL context
current before every draw. On Wayland GDK uses EGL, on the same EGLDisplay
Flutter opens, so it works by accident. On X11 GDK uses GLX, no EGL context
is current, and upstream falls back to software.

## The patch

- The EGL display comes from `eglGetPlatformDisplayEXT` on GDK's native
  display, the call Flutter makes in `fl_opengl_manager.cc`. EGL returns the
  same EGLDisplay for the same arguments, so it is Flutter's, which the
  EGLImage shared with Flutter's texture needs.
- The config is chosen with Flutter's attributes from the same file.
- Before making the isolated context current on the GTK thread, GDK's
  context is released through GDK (`gdk_gl_context_clear_current`) and made
  current again after. A thread has one current context across EGL and
  GLX, and GDK skips rebinding a context it believes is still current.

Reproduce without a Deck: run the app with `GDK_BACKEND=x11` and open a
stream. Patched, the log shows `H/W rendering with isolated EGL context.`

## `Video.pictureBuilder`

Wraps the picture alone, under the subtitles and controls. The player
page zooms and pans the picture with a Flutter `Transform` through it,
which the compositor applies at the screen's rate. Doing it in libmpv
(`video-zoom`, `video-pan-*`) lagged well behind a drag: each change is a
blocking property set and an mpv redraw.

It is a field of `VideoViewParameters`, so media_kit's fullscreen route,
which builds a new `Video` from those, keeps it. Changed:
`lib/src/video_view_parameters.dart`, `lib/src/video/video_texture.dart`,
`lib/src/video/video_web.dart` (accepted, unused) and
`lib/media_kit_video_controls/src/controls/methods/fullscreen.dart`.

## The Windows video output's log

Upstream's Windows video output writes what it does to `std::cout`, which
a Windows GUI app throws away, and never checks its D3D 11 devices. When
one is removed, mpv only says `Mapping hardware decoded surface failed.`
for each frame, and nothing says why.

Patched, `VideoOutput` sends its lines over the method channel
`jitter/media_kit_video/log` (`level`, `message`), which `MediaKitEngine`
writes to the app's log under `media_kit/video`:

- whether it renders in hardware, and why not, and whether ANGLE named
  its D3D 11 device: the one mpv decodes on with `hwdec=d3d11va`;
- each resize, and each texture created and freed;
- each change in what `mpv_render_context_render` returns;
- after each frame, `GetDeviceRemovedReason` on ANGLE's device and on
  media_kit's own, which shares frames with Flutter: the first time
  either stops working, its reason (`DXGI_ERROR_DEVICE_HUNG`, …);
- when the app asks over the same channel (method `checkDevices`, with
  `handle` and `when`), both devices' state at that moment, named by the
  app's checkpoint (a stream ending, the next one loading, its decoder
  starting), so a reset falls between two checkpoints in the log.
- when the app asks over the same channel to flush (method `flush`, with
  `handle` and `when`), it renders once, so libmpv applies the video teardown
  mpv leaves pending on going idle. Without it the next stream reuses mpv's
  d3d11-egl hwdec mapper and ANGLE's D3D 11 device is reset (see
  `docs/windows-d3d11-device-reset.md`).

Changed: `windows/video_output.{h,cc}`, `windows/video_output_manager.{h,cc}`,
`windows/media_kit_video_plugin.{h,cc}` and
`windows/angle_surface_manager.{h,cc}`.
