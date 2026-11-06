#!/usr/bin/env bash
# The patched libmpv jars the Android build needs, in dist/libmpv-android/
# (gitignored). Built here, stored as assets of a GitHub release, fetched
# from it by release.yml and by anyone building for Android.
#
# The pins, libmpv/jars.md5, are the one source of truth: Gradle checks the
# jars against them, and the release's name is a hash of them, so new jars
# always get a new release and old app versions keep theirs.
#
#   scripts/libmpv-android.sh fetch           download the pinned jars
#   scripts/libmpv-android.sh verify          check local jars against the pins
#   scripts/libmpv-android.sh build [dir]     build the jars (an hour or more)
#                                             and pin them
#   scripts/libmpv-android.sh publish         upload the pinned jars
#   scripts/libmpv-android.sh name            print the release's name
#
# Rebuilding: build, publish, then commit libmpv/jars.md5. Publish before
# the commit is pushed, or release.yml finds no release to fetch from.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

repo=${GITHUB_REPOSITORY:-lumpsoid/jitter}
libmpv=third_party/media_kit_libs_android_video/libmpv
pins=$libmpv/jars.md5
jars=dist/libmpv-android

name() {
  echo "libmpv-android-$(sha256sum "$pins" | cut -c1-8)"
}

verify() {
  (cd "$jars" && md5sum --quiet -c "$OLDPWD/$pins")
}

fetch() {
  mkdir -p "$jars"
  gh release download "$(name)" -R "$repo" -p 'default-*.jar' -D "$jars" --clobber
  verify
  echo "Fetched the jars of $(name) into $jars/"
}

build() {
  "$libmpv/build.sh" "$@"
  (cd "$jars" && md5sum default-arm64-v8a.jar default-armeabi-v7a.jar \
    default-x86_64.jar default-x86.jar) > "$pins"
  echo
  git --no-pager diff --stat -- "$pins"
  echo "Pinned as $(name). Next: $0 publish, then commit $pins."
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
    --title "libmpv for Android, $release (build dependency)" \
    --latest=false \
    --notes "Not an app release. media-kit's libmpv-android-video-build with the FFmpeg patches in $libmpv, built by scripts/libmpv-android.sh. The app's Android build fetches these jars; their MD5s are pinned in $pins." \
    "$jars"/default-*.jar "$pins"
  echo "Published $release. Commit $pins if it changed."
}

case "${1:-}" in
  fetch) fetch ;;
  verify) verify && echo "The jars in $jars/ match $(name)" ;;
  build) shift; build "$@" ;;
  publish) publish ;;
  name) name ;;
  *) sed -n '2,/^set -e/{/^set -e/d;s/^# \{0,1\}//;p}' "$0" >&2; exit 2 ;;
esac
