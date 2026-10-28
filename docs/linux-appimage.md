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
