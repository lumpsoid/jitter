# Command runner only: every target is phony. Cargo and Flutter own
# incremental builds; don't model file dependencies here.

.PHONY: check rust-check flutter-check gen fmt run cli appimage

# The full gate. Run before committing.
check: rust-check flutter-check

rust-check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

# Bindings are gitignored, so regenerate before analyzing.
flutter-check: gen
	flutter analyze
	flutter test

gen:
	rinf gen

fmt:
	cargo fmt
	dart format lib test

run: gen
	flutter run -d linux

# Pass CLI arguments with ARGS, e.g. `make cli ARGS="search foo"`.
cli:
	cargo run -p jitter_cli -- $(ARGS)

# Portable Linux build in dist/. Needs podman.
appimage:
	packaging/appimage/build.sh
