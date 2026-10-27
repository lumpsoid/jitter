#!/usr/bin/env bash
# Runs inside the builder container from a clean copy of the source tree.
# Builds the Flutter bundle and wraps it into an AppImage in /out.
set -euo pipefail

flutter pub get
rinf gen
flutter build linux --release

appdir=build/AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin"
# The Flutter runner finds data/ and lib/ next to its executable, so the
# bundle stays intact under usr/bin.
cp -a build/linux/x64/release/bundle/. "$appdir/usr/bin/"
cp android/app/src/main/res/mipmap-xxxhdpi/ic_launcher.png build/jitter.png

# Left to the host, on top of linuxdeploy's built-in excludelist (GL, DRM,
# ALSA, core X11): the GTK/GLib stack the Flutter runner shares with the
# host, and PipeWire, whose library is useless without the host's modules.
excludes=(
  'libgtk-3.so*' 'libgdk-3.so*' 'libgdk_pixbuf-2.0.so*'
  'libglib-2.0.so*' 'libgobject-2.0.so*' 'libgio-2.0.so*' 'libgmodule-2.0.so*'
  'libpango*' 'libcairo*' 'libatk*' 'libatspi*' 'libepoxy.so*'
  'libharfbuzz*' 'libfontconfig.so*' 'libfreetype.so*'
  'libpipewire*' 'libsystemd*' 'libudev*'
)
exclude_args=()
for lib in "${excludes[@]}"; do exclude_args+=(--exclude-library "$lib"); done

# libmpv and libavdevice link libjack, which linuxdeploy's excludelist leaves
# to the host, but plenty of desktops don't have it; without it the app does
# not start at all. Playback never goes through JACK, so a bundled copy that
# doesn't match the host's JACK server costs nothing.
mkdir -p "$appdir/usr/lib"
cp -L /usr/lib/x86_64-linux-gnu/{libjack.so.0,libdb-5.3.so} "$appdir/usr/lib/"

version=$(sed -n 's/^version: *\([^+]*\).*/\1/p' pubspec.yaml)
LINUXDEPLOY_OUTPUT_VERSION="$version" APPIMAGE_EXTRACT_AND_RUN=1 linuxdeploy \
  --appdir "$appdir" \
  --executable "$appdir/usr/bin/jitter" \
  --desktop-file packaging/appimage/jitter.desktop \
  --icon-file build/jitter.png \
  "${exclude_args[@]}" \
  --output appimage

cp Jitter-*.AppImage /out/
