#!/bin/bash
# Rebuilds android/libs/default-*.jar: media-kit's libmpv-android-video-build
# at v1.1.7 (the release media_kit_libs_android_video 1.3.8 pins) plus
# mov_reset_fragments.patch. Prints the MD5s to pin in android/build.gradle.
#
# Needs: JDK 17, nasm, meson, ninja, autotools, wget, unzip, and an Android
# SDK at $ANDROID_HOME with cmdline-tools; the scripts install NDK
# 25.2.9519653, build-tools 33.0.2, platform 33 and cmake 3.22.1 into it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work="${1:-/tmp/jitter-libmpv-build}"
: "${ANDROID_HOME:?set ANDROID_HOME to an Android SDK}"

if [ ! -d "$work" ]; then
  git clone --branch v1.1.7 --depth 1 \
    https://github.com/media-kit/libmpv-android-video-build.git "$work"
fi
cd "$work/buildscripts"

cp "$here/mov_reset_fragments.patch" patches/ffmpeg/
# Upstream's scripts assume a CI runner: drop sudo, and strip with the NDK
# the build uses rather than the one the runner happened to have.
sed -i 's/sudo //' bundle_default.sh build.sh
sed -i 's#ndk/27\.3\.13750724/#ndk/25.2.9519653/#' bundle_default.sh

mkdir -p sdk
ln -sfn "$ANDROID_HOME" sdk/android-sdk-linux

./bundle_default.sh

out=deps/media-kit-android-helper/app/build/outputs/apk/release
cp "$out"/default-*.jar "$here/../android/libs/"
cd "$here/../android/libs" && md5sum default-*.jar
