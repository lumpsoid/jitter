#!/usr/bin/env bash
# The app version lives in two files that must agree: Cargo.toml's
# [workspace.package] and pubspec.yaml (as X.Y.Z+N, N being Android's
# versionCode, which must grow every release).
#
#   scripts/version.sh show           print X.Y.Z+N
#   scripts/version.sh check          fail unless both files agree
#   scripts/version.sh bump X.Y.Z     set both, N += 1, date the changelog's
#                                     [Unreleased] section as X.Y.Z, and
#                                     refresh Cargo.lock
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

repo_url=https://github.com/lumpsoid/jitter

cargo_version() {
  sed -n '/^\[workspace\.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml
}

pubspec_full() {
  sed -n 's/^version: *\([^ ]*\).*/\1/p' pubspec.yaml
}

check() {
  local cargo pub
  cargo=$(cargo_version)
  pub=$(pubspec_full)
  if [[ -z $cargo || ! $pub =~ ^[0-9]+\.[0-9]+\.[0-9]+\+[0-9]+$ ]]; then
    echo "cannot read the version: Cargo.toml '$cargo', pubspec.yaml '$pub'" >&2
    exit 1
  fi
  if [[ ${pub%+*} != "$cargo" ]]; then
    echo "versions differ: Cargo.toml $cargo, pubspec.yaml ${pub%+*}" >&2
    exit 1
  fi
}

bump() {
  local new=$1 code today previous
  if [[ ! $new =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "not a version: $new (want X.Y.Z)" >&2
    exit 2
  fi
  check
  previous=$(cargo_version)
  code=$(( $(pubspec_full | cut -d+ -f2) + 1 ))
  today=$(date +%F)
  if ! grep -q '^## \[Unreleased\]' CHANGELOG.md; then
    echo "CHANGELOG.md has no ## [Unreleased] section" >&2
    exit 1
  fi
  if grep -q "^## \[$new\]" CHANGELOG.md; then
    echo "CHANGELOG.md already has a $new section" >&2
    exit 1
  fi

  sed -i "/^\[workspace\.package\]/,/^\[/s/^version = \".*\"/version = \"$new\"/" Cargo.toml
  sed -i "s/^version: .*/version: $new+$code/" pubspec.yaml

  # A fresh, empty [Unreleased] above the dated section, and the links at
  # the foot: [Unreleased] compares from the new tag, the new version from
  # the previous one (or is the tag itself, for the first release).
  sed -i "s/^## \[Unreleased\]$/## [Unreleased]\n\n## [$new] - $today/" CHANGELOG.md
  sed -i "s#^\[Unreleased\]: .*#[Unreleased]: $repo_url/compare/v$new...HEAD#" CHANGELOG.md
  if git rev-parse -q --verify "refs/tags/v$previous" >/dev/null; then
    echo "[$new]: $repo_url/compare/v$previous...v$new" >> CHANGELOG.md
  else
    echo "[$new]: $repo_url/releases/tag/v$new" >> CHANGELOG.md
  fi

  cargo update --workspace --quiet
  check
  echo "$new+$code"
}

case "${1:-}" in
  show) check; pubspec_full ;;
  check) check ;;
  bump) bump "${2:?usage: $0 bump X.Y.Z}" ;;
  *) echo "usage: $0 show|check|bump X.Y.Z" >&2; exit 2 ;;
esac
