#!/usr/bin/env bash
# Builds dist/Jitter-<version>-x86_64.AppImage in an Ubuntu 24.04 container.
# The source goes in as a tarball of tracked and untracked-but-not-ignored
# files, so host build artifacts (CMake caches, .dart_tool) never leak in.
# It has no .git, so the commit the app shows goes in as JITTER_COMMIT.
set -euo pipefail

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$root"

podman build -t jitter-appimage-builder packaging/appimage
mkdir -p dist

git ls-files -co --exclude-standard -z \
  | tar --null -T - -c \
  | podman run --rm -i \
      -e JITTER_COMMIT="$(git rev-parse --short=7 HEAD)" \
      -v jitter-appimage-cargo:/root/.cargo/registry \
      -v jitter-appimage-pub:/root/.pub-cache \
      -v jitter-appimage-target:/work/target \
      -v "$root/dist:/out:Z" \
      jitter-appimage-builder \
      bash -c 'tar -x && bash packaging/appimage/inside.sh'

ls -lh dist/*.AppImage
