#!/usr/bin/env bash
# Tags a release of the version in Cargo.toml and pubspec.yaml after every
# gate passes. Run after `scripts/version.sh bump X.Y.Z` is committed.
# Never pushes: it prints the push, and the tag's workflow builds and
# publishes the release.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

fail() {
  echo "$*" >&2
  exit 1
}

[[ $(git branch --show-current) == main ]] || fail "not on main"
[[ -z $(git status --porcelain) ]] || fail "the working tree is not clean"
scripts/version.sh check
version=$(scripts/version.sh show)
version=${version%+*}
tag="v$version"
git rev-parse -q --verify "refs/tags/$tag" >/dev/null && fail "$tag already exists"

# The section's body, without its heading, up to the next section or the
# link references at the foot.
notes=$(awk -v head="## [$version]" '
  index($0, head) == 1 { on = 1; next }
  on && (/^## \[/ || /^\[[^]]+\]: /) { exit }
  on { print }
' CHANGELOG.md | sed -e '/./,$!d' | sed -e ':a' -e '/^\n*$/{$d;N;ba' -e '}')
[[ -n $notes ]] || fail "CHANGELOG.md has no notes under ## [$version]"

scripts/check.sh

# verbatim: the default cleanup would drop the "### Added" lines as comments.
git tag -a --cleanup=verbatim -m "jitter $version" -m "$notes" "$tag"
echo
echo "Tagged $tag. Push it with:"
echo "  git push origin main $tag"
