# media_kit_video, patched

`media_kit_video` 2.0.1, vendored without `example/`. Two patches differ
from upstream: hardware rendering on X11 (`linux/video_output.cc`), and
`Video.pictureBuilder` (below).

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
