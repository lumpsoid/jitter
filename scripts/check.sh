#!/usr/bin/env bash
# Every gate, in CI's order; stops at the first failure.
#
#   scripts/check.sh            all of them
#   scripts/check.sh rust       versions agree, cargo fmt, clippy, test
#   scripts/check.sh flutter    rinf gen, flutter analyze, flutter test
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

step() {
  echo "==> $*"
  "$@"
}

rust() {
  step scripts/version.sh check
  step cargo fmt --check
  step cargo clippy --all-targets -- -D warnings
  step cargo test
}

# The bindings are gitignored, so they are generated before analyzing.
flutter_gates() {
  step rinf gen
  step flutter analyze
  step flutter test
}

case "${1:-all}" in
  rust) rust ;;
  flutter) flutter_gates ;;
  all) rust && flutter_gates ;;
  *) echo "usage: $0 [rust|flutter|all]" >&2; exit 2 ;;
esac
