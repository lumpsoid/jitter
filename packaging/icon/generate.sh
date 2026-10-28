#!/usr/bin/env bash
# Generates every platform's app icon from one design: the letter J sliced
# into horizontal bands that slip sideways, like a dropped video frame.
#
# Writes the source SVGs next to this script, then rasterises them into the
# Android, iOS, macOS, Linux and Windows runner trees. Needs rsvg-convert and magick.
#
#   packaging/icon/generate.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"

BG='#4c1d95'
FG='#ffffff'
ACCENT='#22d3ee'

# The J on a 100×100 grid, cut into bands: "top height shift colour".
bands() {
  local fg=$1 accent=$2 k=0
  while read -r y h dx colour; do
    [ "$colour" = fg ] && colour=$fg || colour=$accent
    cat <<EOF
  <clipPath id="b$k"><rect x="0" y="$y" width="100" height="$h"/></clipPath>
  <g clip-path="url(#b$k)"><path transform="translate($dx 0)" d="M42 24 H66 V58 A16 16 0 0 1 34 58" fill="none" stroke="$colour" stroke-width="13" stroke-linecap="round" stroke-linejoin="round"/></g>
EOF
    k=$((k + 1))
  done <<'EOF'
0 30 0 fg
30 9 6 fg
39 14 -3 fg
53 7 8 accent
60 14 0 fg
74 6 -6 fg
80 20 0 fg
EOF
}

# Full-bleed square: iOS masks it itself.
cat >"$here/jitter.svg" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <rect width="100" height="100" fill="$BG"/>
$(bands "$FG" "$ACCENT")
</svg>
EOF

# Self-shaped rounded square: Android legacy, Windows, Linux.
cat >"$here/jitter-rounded.svg" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <rect width="100" height="100" rx="22.5" fill="$BG"/>
$(bands "$FG" "$ACCENT")
</svg>
EOF

# macOS: rounded square inset on Apple's 1024 grid (824 body), with a shadow.
cat >"$here/jitter-macos.svg" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <filter id="s" x="-10%" y="-10%" width="120%" height="130%">
    <feDropShadow dx="0" dy="10" stdDeviation="12" flood-opacity="0.3"/>
  </filter>
  <g transform="translate(100 100) scale(8.24)">
    <rect width="100" height="100" rx="22.5" fill="$BG" filter="url(#s)"/>
$(bands "$FG" "$ACCENT")
  </g>
</svg>
EOF

# Android adaptive layers: 108dp canvas, J inside the 66dp safe circle.
cat >"$here/jitter-foreground.svg" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" viewBox="-25 -25 150 150">
$(bands "$FG" "$ACCENT")
</svg>
EOF
cat >"$here/jitter-monochrome.svg" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" viewBox="-25 -25 150 150">
$(bands '#ffffff' '#ffffff')
</svg>
EOF
# Android notification small icon: Android draws only its alpha, so the J
# alone, cropped close to fill the 24dp status-bar slot.
cat >"$here/jitter-notification.svg" <<EOF
<svg xmlns="http://www.w3.org/2000/svg" viewBox="15 13 72 72">
$(bands '#ffffff' '#ffffff')
</svg>
EOF

png() { rsvg-convert -w "$2" -h "$2" "$here/$1" -o "$3"; }

# Android
res="$root/android/app/src/main/res"
for d in mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
  density=${d%%:*} px=${d##*:} layer=$((px * 9 / 4))
  png jitter-rounded.svg "$px" "$res/mipmap-$density/ic_launcher.png"
  png jitter-foreground.svg "$layer" "$res/mipmap-$density/ic_launcher_foreground.png"
  png jitter-monochrome.svg "$layer" "$res/mipmap-$density/ic_launcher_monochrome.png"
  mkdir -p "$res/drawable-$density"
  png jitter-notification.svg "$((px / 2))" "$res/drawable-$density/ic_stat_jitter.png"
done
# audio_service looks the notification icon up by name, so resource
# shrinking cannot see it is used.
mkdir -p "$res/raw"
cat >"$res/raw/keep.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<resources xmlns:tools="http://schemas.android.com/tools"
    tools:keep="@drawable/ic_stat_jitter"/>
EOF
mkdir -p "$res/mipmap-anydpi-v26"
cat >"$res/mipmap-anydpi-v26/ic_launcher.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome"/>
</adaptive-icon>
EOF
cat >"$res/values/ic_launcher_background.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">$BG</color>
</resources>
EOF

# iOS: sizes come from the existing file names (Icon-App-WxH@Sx.png).
# App Store rejects icons with an alpha channel.
for f in "$root"/ios/Runner/Assets.xcassets/AppIcon.appiconset/Icon-App-*.png; do
  name=$(basename "$f" .png)
  side=${name#Icon-App-}; side=${side%%x*}
  scale=${name##*@}; scale=${scale%x}
  px=$(awk "BEGIN { print int($side * $scale + 0.5) }")
  png jitter.svg "$px" "$f"
  magick "$f" -background "$BG" -alpha remove -alpha off -define png:exclude-chunks=date,time "$f"
done

# macOS
for f in "$root"/macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_*.png; do
  px=$(basename "$f" .png); px=${px#app_icon_}
  png jitter-macos.svg "$px" "$f"
done

# Linux: the runner loads this as its window icon; the AppImage reuses it.
mkdir -p "$root/linux/runner/resources"
png jitter-rounded.svg 256 "$root/linux/runner/resources/app_icon.png"

# Windows
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for px in 16 24 32 48 64 128 256; do png jitter-rounded.svg "$px" "$tmp/$px.png"; done
magick "$tmp"/{16,24,32,48,64,128,256}.png "$root/windows/runner/resources/app_icon.ico"

echo "icons written"
