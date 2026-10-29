#!/usr/bin/env bash
# Runs inside the builder container from a clean copy of the source tree.
# Builds the AppImage into /out.
set -euo pipefail

# libmpv and libavdevice link libjack, which linuxdeploy's excludelist leaves
# to the host, but plenty of desktops don't have it; without it the app does
# not start at all. Playback never goes through JACK, so a bundled copy that
# doesn't match the host's JACK server costs nothing. Ubuntu's is jack2's,
# which needs libdb.
exec bash packaging/appimage/assemble.sh /out "" \
  /usr/lib/x86_64-linux-gnu/{libjack.so.0,libdb-5.3.so}
