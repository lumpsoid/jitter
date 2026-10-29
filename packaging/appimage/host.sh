#!/usr/bin/env bash
# Builds dist/Jitter-<version>-host-x86_64.AppImage on this machine, from
# its own libmpv and ffmpeg, in the working tree. For testing: the
# container build (build.sh) is the portable one. Written for Arch: needs
# mpv, patchelf, python, flutter, cargo and rinf installed.
#
# glibc cannot be bundled, and Arch's libraries ask for the newest glibc.
# The build lowers that to GLIBC_TARGET (default 2.43, SteamOS's in October
# 2026) where glibc keeps an older version of the symbol
# (glibc_compat.py), and stops where it cannot. That holds only as long as
# Arch's libraries use nothing newer; an update can break it again.
set -euo pipefail

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$root"

export GLIBC_TARGET=${GLIBC_TARGET:-2.43}

if [[ ! -e /usr/lib/libmpv.so.2 ]]; then
  echo "libmpv not found: install mpv (pacman -S mpv)" >&2
  exit 1
fi

# linuxdeploy is itself an AppImage; extracted once into build/, so it runs
# without FUSE. `flutter clean` removes it and the next build fetches it.
tools=build/linuxdeploy
if [[ ! -x $tools/AppRun ]]; then
  mkdir -p build
  curl -sSfL -o build/linuxdeploy.AppImage \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
  chmod +x build/linuxdeploy.AppImage
  rm -rf build/squashfs-root "$tools"
  (cd build && ./linuxdeploy.AppImage --appimage-extract >/dev/null)
  mv build/squashfs-root "$tools"
  rm build/linuxdeploy.AppImage
fi
mkdir -p build/bin
ln -sf "$root/$tools/AppRun" build/bin/linuxdeploy
export PATH="$root/build/bin:$PATH"

# Bundled despite linuxdeploy's excludelist:
# - libjack, as in inside.sh: libmpv links it and many hosts lack it. On
#   Arch it is usually pipewire-jack's, which needs only the host's
#   libpipewire, left to the host anyway.
# - libstdc++: Arch's libraries (libopenal) ask for a newer one than
#   SteamOS has. It is backward compatible, so a newer copy also serves
#   the host's GL drivers; on a host with a newer one still, those drivers
#   could fail to load.
bash packaging/appimage/assemble.sh dist -host \
  /usr/lib/libjack.so.0 /usr/lib/libstdc++.so.6

ls -lh dist/Jitter-*-host-*.AppImage
# The newest glibc symbol version anything in it asks for, as built.
glibc=$(find build/AppDir/usr -type f \( -name '*.so*' -o -name jitter \) \
  -exec objdump -T {} + 2>/dev/null \
  | grep -o 'GLIBC_2\.[0-9]*' | sort -t. -k2 -n -u | tail -1)
echo "Needs ${glibc/GLIBC_/glibc } or newer (check with ldd --version)."
