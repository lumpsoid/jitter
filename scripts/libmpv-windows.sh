#!/usr/bin/env bash
# The libmpv archive the Windows build needs, in dist/libmpv-windows/
# (gitignored): shinchiro's mpv-dev build of mpv and FFmpeg from git, taken
# from its SourceForge mirror, stored as an asset of a GitHub release, and
# downloaded from it by media_kit_libs_windows_video's CMakeLists.txt.
#
# The pin, libmpv/archive.md5, is the one source of truth: CMake checks the
# archive against it, and the release's name is the start of that MD5, so a
# new build always gets a new release and old app versions keep theirs.
#
#   scripts/libmpv-windows.sh latest          print the newest build on
#                                             SourceForge
#   scripts/libmpv-windows.sh update [date]   download a build (default: the
#                                             newest), check it and pin it
#   scripts/libmpv-windows.sh verify          check the local archive against
#                                             the pin
#   scripts/libmpv-windows.sh publish         upload the pinned archive
#   scripts/libmpv-windows.sh name            print the release's name
#
# Updating: update, publish, then commit libmpv/archive.md5 and test the
# Windows build. Publish before the commit is pushed, or the Windows build
# finds no release to download from.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

repo=${GITHUB_REPOSITORY:-lumpsoid/jitter}
libmpv=third_party/media_kit_libs_windows_video/libmpv
pin=$libmpv/archive.md5
dist=dist/libmpv-windows
sf=https://sourceforge.net/projects/mpv-player-windows
sf_files=https://downloads.sourceforge.net/project/mpv-player-windows/libmpv

name() {
  echo "libmpv-windows-$(cut -c1-8 "$pin")"
}

pinned() {
  cut -d' ' -f3 "$pin"
}

verify() {
  (cd "$dist" && md5sum --quiet -c "$OLDPWD/$pin")
}

# "<md5>  <file>" per x86_64 mpv-dev build on SourceForge, newest first.
# SourceForge publishes each file's MD5 in the folder's RSS feed.
builds() {
  curl -fsSL "$sf/rss?path=/libmpv" | python3 -I -c '
import re, sys, xml.etree.ElementTree as ET
rows = []
for item in ET.parse(sys.stdin).iter("item"):
    file = item.findtext("title", "").rsplit("/", 1)[-1]
    if not re.fullmatch(r"mpv-dev-x86_64-\d{8}-git-[0-9a-f]+\.7z", file):
        continue
    md5 = [e.text for e in item.iter()
           if e.tag.endswith("}hash") and e.get("algo") == "md5"]
    if md5:
        rows.append((file, md5[0].strip()))
for file, md5 in sorted(rows, reverse=True):
    print(f"{md5}  {file}")
'
}

latest() {
  builds | head -n1 | cut -d' ' -f3
}

update() {
  local date=${1:-} row
  if [ -n "$date" ]; then
    row=$(builds | grep -E "  mpv-dev-x86_64-$date-git-" | head -n1) || true
  else
    row=$(builds | head -n1)
  fi
  if [ -z "$row" ]; then
    echo "no mpv-dev-x86_64 build ${date:+for $date }in $sf/files/libmpv/" >&2
    exit 1
  fi
  local file=${row##* }
  mkdir -p "$dist"
  curl -fL --progress-bar -o "$dist/$file" "$sf_files/$file"
  # Check against SourceForge's MD5 before pinning it.
  (cd "$dist" && echo "$row" | md5sum --quiet -c -)
  # The files the build links against and bundles.
  local listing
  listing=$(7z l -ba "$dist/$file")
  for f in libmpv-2.dll libmpv.dll.a include/mpv/client.h include/mpv/render_gl.h; do
    if ! grep -qE "[ /]$f\$" <<<"$listing"; then
      echo "$file has no $f" >&2
      exit 1
    fi
  done
  echo "$row" > "$pin"
  echo
  git --no-pager diff --stat -- "$pin"
  echo "Pinned $file as $(name). Next: $0 publish, then commit $pin."
}

publish() {
  verify
  local release
  release=$(name)
  if gh release view "$release" -R "$repo" >/dev/null 2>&1; then
    echo "$release is already published"
    return
  fi
  gh release create "$release" -R "$repo" \
    --title "libmpv for Windows, $release (build dependency)" \
    --latest=false \
    --notes "Not an app release. $(pinned), shinchiro's mpv-winbuild-cmake build of mpv and FFmpeg, unchanged, from $sf/files/libmpv/. The app's Windows build downloads it; its MD5 is pinned in $pin." \
    "$dist/$(pinned)" "$pin"
  echo "Published $release. Commit $pin if it changed."
}

case "${1:-}" in
  latest) latest ;;
  update) shift; update "$@" ;;
  verify) verify && echo "$dist/$(pinned) matches $(name)" ;;
  publish) publish ;;
  name) name ;;
  *) sed -n '2,/^set -e/{/^set -e/d;s/^# \{0,1\}//;p}' "$0" >&2; exit 2 ;;
esac
