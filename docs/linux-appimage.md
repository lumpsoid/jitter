# Linux AppImage

How to build the portable Linux build and run it on another machine,
including SteamOS. The why behind the packaging choices is in the commit
that added `packaging/appimage/`.

## Why not the plain bundle

`flutter build linux` produces `build/linux/x64/release/bundle/`, but
media_kit links the system libmpv. On a machine without mpv the bundle fails
with:

```
./jitter: error while loading shared libraries: libmpv.so.2: cannot open shared object file
```

The AppImage carries libmpv and its ffmpeg stack inside it.

## Build

```
make appimage
```

Needs podman. The build runs in an Ubuntu 24.04 container and writes
`dist/Jitter-<version>-x86_64.AppImage`. The first run builds the image
(Flutter, Rust, rinf, linuxdeploy) and takes a while; later runs reuse it.

The AppImage needs **glibc 2.38 or newer** on the target: Ubuntu 24.04+,
Debian 13, current Fedora, Arch, SteamOS 3.x. Check with `ldd --version`.

### On this machine instead

```
make appimage-host
```

Builds on the host from its own libmpv and ffmpeg, without podman, and
writes `dist/Jitter-<version>-host-x86_64.AppImage`. Written for Arch: needs
`mpv`, `flutter`, `cargo` and `rinf`; the first run fetches linuxdeploy into
`build/`.

It is for testing. glibc can't be bundled, and Arch's libraries ask for the
newest one. The build lowers that to `GLIBC_TARGET` (default 2.43, SteamOS's
in October 2026) where glibc keeps an older version of the symbol: glibc
2.44's `cosh` and `sinh` (`packaging/appimage/glibc_compat.py`). It stops,
naming the symbols, where it can't. It also bundles Arch's `libstdc++`, which
`libopenal` needs newer than SteamOS's. The script prints the glibc the
result needs.

An Arch update can bring another symbol only the newest glibc has, and break
the build again. For a build to hand out, use `make appimage`. What the host
build gains is the host's ffmpeg: Arch's is 9.0+, which seeks in a VOD that
is still recording (see below).

## Run

```
chmod +x Jitter-<version>-x86_64.AppImage   # if the copy dropped the exec bit
./Jitter-<version>-x86_64.AppImage
```

If it reports `Cannot mount AppImage, please check your FUSE setup`, run it
without mounting:

```
./Jitter-<version>-x86_64.AppImage --appimage-extract-and-run
```

### A library is missing

GTK, GLib, PipeWire, GL and the usual desktop libraries come from the host.
To list what the host lacks:

```
./Jitter-<version>-x86_64.AppImage --appimage-extract >/dev/null
for f in squashfs-root/usr/bin/jitter squashfs-root/usr/lib/*.so*; do
  ldd "$f" | grep "not found"
done | sort -u
```

### Seeking in a VOD that is still recording

Known limitation. The ffmpeg inside the AppImage is Ubuntu 24.04's 6.1, which
lacks the fix for seeking in HLS EVENT playlists (FFmpeg `6d98a9a2e8`, first
released in 9.0). So when "Rewind stream" opens the live broadcast's
recording, or you open a VOD that is still recording from a channel's archive,
playback starts at the recording's live edge and every seek snaps back there.
Fixing it is a packaging change: build FFmpeg 9.0 or newer, or the backport
patch Android carries, in the container. The host build (`make
appimage-host`) on a machine with ffmpeg 9.0+ does not have this limitation.

## SteamOS

Don't install anything with `pacman`: the system partition is read-only and
updates wipe it. Everything Jitter needs is in the AppImage or on SteamOS.

### Desktop Mode

Copy the AppImage to `~/Applications/`, make it executable (Dolphin:
Properties → Permissions → *Is executable*, or `chmod +x`), and run it.

### Game Mode

Steam launches non-Steam games without working FUSE, so the AppImage cannot
mount itself and Steam drops straight back to the library. Extract it once
and point Steam at the extracted tree instead.

In Desktop Mode, in Konsole:

```
cd ~/Applications
./Jitter-<version>-x86_64.AppImage --appimage-extract
mv squashfs-root jitter
```

Then in Steam: *Games → Add a Non-Steam Game to My Library… → Browse*, set
the file filter to *All files*, and add `jitter/AppRun`. In the shortcut's
Properties:

- **Target:** `/home/deck/Applications/jitter/AppRun`
- **Start In:** `/home/deck/Applications/jitter`
- **Compatibility:** leave *Force the use of a specific Steam Play
  compatibility tool* unticked; Proton cannot run a Linux binary.

To update, delete `~/Applications/jitter` and extract the new AppImage the
same way.

### Touch acts as a mouse

By default Steam turns taps on the touchscreen into mouse clicks: a cursor
appears under your finger and Jitter shows its mouse controls. To pass touch
through instead, with Jitter running in Game Mode:

1. Press the Steam button, then *Controller settings → Edit layout*.
2. Open *Action sets → Default → Add Always-On Command*.
3. Select the new command, then *System → Touchscreen Native Support*.
4. Save the layout.

The layout belongs to the shortcut, so this is needed once.

### Debugging a Game Mode launch

Game Mode hides the app's output. To capture it, create
`~/Applications/jitter-steam.sh`:

```bash
#!/bin/bash
exec > "$HOME/jitter-steam.log" 2>&1
echo "== $(date)"
env | sort
echo "== running"
# Line-buffered: Steam kills the app on exit, losing buffered output.
stdbuf -oL -eL "$HOME/Applications/jitter/AppRun" "$@"
echo "== exit code $?"
```

`chmod +x` it, set it as the shortcut's Target, launch from Game Mode, and
read `~/jitter-steam.log`: Steam's environment followed by whatever the app
printed before exiting.
