# Workaround: VA-API only under gamescope

**Status:** active since `58600d6` (2026-11-02). Remove it once SteamOS ships
the fix (see [Removing it](#removing-it)).

**Code:** `hwdecFor` in `lib/player/media_kit_engine.dart`, tested by
`test/player/hwdec_test.dart`.

## Symptom

In SteamOS game mode, Jitter aborts (exit 134) as soon as a stream's video
output reaches its full size. The log ends with:

```
[Gamescope WSI] Application info:
  pApplicationName: ffmpeg
  applicationVersion: 255856742
  pEngineName: libavutil
  ...
AppRun: ../subprojects/vkroots/vkroots.h:129: const DispatchType* vkroots::tables::VkDispatchTableMap<Object, DispatchType, DispatchPtr>::insert(Object, DispatchPtr) [with Object = VkQueue_T*; ...]: Assertion `obj' failed.
... Aborted (core dumped)
```

Desktop mode is not affected: gamescope's WSI layer is only loaded when Steam
sets `ENABLE_GAMESCOPE_WSI=1`, which it does in game mode.

## Cause

1. media_kit sets mpv's `hwdec` to `auto` by default
   (`third_party/media_kit_video/lib/src/video_controller/native_video_controller/real.dart`).
   `auto` also tries Vulkan decoding, so FFmpeg creates a Vulkan device.
2. Since FFmpeg 9.0, `libavutil/hwcontext_vulkan.c` creates every queue with
   `VK_DEVICE_QUEUE_CREATE_INTERNALLY_SYNCHRONIZED_BIT_KHR` when the driver
   offers `VK_KHR_internally_synchronized_queues`. Mesa 26.1 (RADV, the
   Steam Deck's driver) offers it.
3. Gamescope's WSI layer is built on vkroots. After `vkCreateDevice`,
   vkroots' `tables::CreateDispatchTable` fetches each queue with
   `vkGetDeviceQueue`. Per the Vulkan spec, that call only returns queues
   created with `flags == 0`, so for flagged queues it gets `VK_NULL_HANDLE`.
   It then passes that handle to `AssignDispatchTable`, where
   `assert(obj)` aborts the process.

`applicationVersion` is `VK_MAKE_VERSION` of libavutil's version, which
tells you which FFmpeg the build bundles:

| applicationVersion | libavutil | FFmpeg | Crashes |
|---|---|---|---|
| 243388516 | 58.29.100 | 6.1 | no, no queue flag |
| 255856742 | 61.1.102 | 9.0 | yes |

The FFmpeg 9 build came in with the AppImage built on the Arch host
(`72969c9`, see [linux-appimage](./linux-appimage.md)).

## Workaround

Under gamescope (`GAMESCOPE_WAYLAND_DISPLAY` set), set `hwdec` to
`vaapi,vaapi-copy`:

- `vaapi` decodes on the GPU and hands frames straight to media_kit's EGL
  renderer. It creates no Vulkan device.
- `vaapi-copy` keeps hardware decoding when media_kit falls back to
  software rendering ("EGL display or context is invalid").
- If neither works, mpv decodes in software. That costs CPU but doesn't
  crash.

Outside gamescope, `auto` stays, so NVDEC still works on desktop NVIDIA.

What this costs: under gamescope there is no Vulkan decoding and no NVDEC.
On the Deck nothing is lost. On gamescope with NVIDIA (e.g. Bazzite), video
falls back to software decoding.

## Removing it

The fix has to land in each of these, in order:

1. **vkroots** merges the fix: misyltoad/vkroots#17, open as of 2026-11-02.
2. **gamescope** bumps its `subprojects/vkroots` submodule to a commit that
   contains it (tracked in ValveSoftware/gamescope#2261). As of
   2026-11-02, gamescope `3.17.0-10-g309eee74` pins vkroots `20733b9`
   (2026-09-08), which doesn't contain the fix.
3. **SteamOS** stable ships that gamescope.

To check whether a SteamOS release has it:

```sh
# On the Deck:
gamescope --version                 # e.g. 3.17.x

# In a gamescope clone, at that tag:
git checkout <tag>
git ls-tree HEAD subprojects/vkroots  # the pinned vkroots commit
# In a vkroots clone: does that commit contain the fix?
git merge-base --is-ancestor <fix commit> <pinned commit> && echo fixed
```

Then confirm on the Deck: delete `hwdecFor` (and its use in the
`VideoController` configuration and `test/player/hwdec_test.dart`), build an
AppImage, and play a stream in game mode. A `[Gamescope WSI] ... ffmpeg`
block followed by no abort means it's fixed.

To check on the Deck without a rebuild, launch with
`ENABLE_GAMESCOPE_WSI=0`. That unloads the layer, so if the crash is gone
with it, the layer is still the cause.

## References

- **vkroots fix:** https://github.com/misyltoad/vkroots/pull/17. It fetches
  queues with `vkGetDeviceQueue2` and the create flags, in both
  `CreateDispatchTable` and `DestroyDispatchTable`, and edits
  `gen/inc/vkroots_dispatches.h` and the generated `vkroots.h`.
- **vkroots repo:** https://github.com/misyltoad/vkroots (gamescope's
  `.gitmodules` still uses the old URL, https://github.com/Joshua-Ashton/vkroots).
- **gamescope tracking issue:**
  https://github.com/ValveSoftware/gamescope/issues/2261
  ("WSI layer aborts (vkroots assert) for apps using
  VK_KHR_internally_synchronized_queues").
- **gamescope duplicate:** https://github.com/ValveSoftware/gamescope/issues/2263
  (Moonlight in game mode, worked around with `ENABLE_GAMESCOPE_WSI=0`).
- **Same crash, other apps:**
  https://github.com/moonlight-stream/moonlight-qt/issues/1925,
  https://github.com/moonlight-stream/moonlight-qt/issues/1930 (libplacebo,
  which sets the same flag since videolan/libplacebo!809).
- **FFmpeg commits that set the flag** (first in `n9.0`):
  - `6f811ad751bf79f0b0a89ebfcc47288d6ebda1bb` (2026-04-21)
    "hwcontext_vulkan: implement internal queue synchronization"
  - `5134b0aceba7d019c3cb27381801afdd81352a99` (2026-04-24)
    "libavutil/hwcontext_vulkan: fix internal queue sync device creation"
- **Vulkan spec:** `vkGetDeviceQueue` is only valid for queues created with
  `flags == 0`; flagged queues need `vkGetDeviceQueue2`.
  https://registry.khronos.org/vulkan/specs/latest/man/html/vkGetDeviceQueue.html
- **mpv `hwdec`:** https://mpv.io/manual/stable/#options-hwdec
