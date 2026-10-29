#!/usr/bin/env bash
# Builds the Flutter bundle in the current source tree and wraps it into
# an AppImage in <out-dir>. Shared by the container build (inside.sh) and
# the host build (host.sh); linuxdeploy must be on PATH, and patchelf too
# where a library is linked by absolute path (see below).
#
#   assemble.sh <out-dir> <name-suffix> [<library to bundle>...]
#
# <name-suffix> goes after the version in the file name ("" for none).
# The libraries are copied into usr/lib as they are: ones linuxdeploy's
# excludelist would leave to the host but the app cannot start without.
set -euo pipefail

mkdir -p "$1"
out=$(realpath "$1")
suffix=$2
shift 2
bundled=("$@")

flutter pub get
rinf gen
flutter build linux --release

appdir=$PWD/build/AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/lib"
# The Flutter runner finds data/ and lib/ next to its executable, so the
# bundle stays intact under usr/bin.
cp -a build/linux/x64/release/bundle/. "$appdir/usr/bin/"
cp linux/runner/resources/app_icon.png build/jitter.png
if ((${#bundled[@]})); then cp -L "${bundled[@]}" "$appdir/usr/lib/"; fi

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

version=$(sed -n 's/^version: *\([^+]*\).*/\1/p' pubspec.yaml)
# linuxdeploy writes the AppImage to its working directory: build/, so a
# host build leaves none in the source tree.
cd build
rm -f Jitter-*.AppImage
deploy() {
  LINUXDEPLOY_OUTPUT_VERSION="$version$suffix" APPIMAGE_EXTRACT_AND_RUN=1 \
    linuxdeploy \
    --appdir "$appdir" \
    --executable "$appdir/usr/bin/jitter" \
    --desktop-file ../packaging/appimage/jitter.desktop \
    --icon-file jitter.png \
    "${exclude_args[@]}" \
    "$@"
}
deploy

# A library built without a SONAME (Arch's libmujs) is recorded by its
# absolute path, which the loader opens as given, never looking in the
# AppImage, and which linuxdeploy cannot resolve, so never bundles. Point
# each such entry at the bare name and bundle the file; the output pass
# deploys its own dependencies and sets its rpath.
absolute=()
while IFS= read -r -d '' elf; do
  while read -r needed; do
    patchelf --replace-needed "$needed" "$(basename "$needed")" "$elf"
    absolute+=("$needed")
  done < <(readelf -d "$elf" 2>/dev/null \
    | sed -n 's/.*(NEEDED).*\[\(\/[^]]*\)\]/\1/p')
done < <(find "$appdir/usr" -type f -name '*.so*' -print0)
if ((${#absolute[@]})); then
  printf '%s\n' "${absolute[@]}" | sort -u | while read -r lib; do
    cp -L "$lib" "$appdir/usr/lib/"
  done
fi

# Given GLIBC_TARGET (host.sh), point what asks for a newer glibc at the
# older versions glibc keeps, or stop (glibc_compat.py).
if [[ -n ${GLIBC_TARGET:-} ]]; then
  find "$appdir/usr" -type f \( -name '*.so*' -o -name jitter \) -print0 \
    | xargs -0 python3 ../packaging/appimage/glibc_compat.py "$GLIBC_TARGET"
fi

deploy --output appimage

cp Jitter-*.AppImage "$out/"
