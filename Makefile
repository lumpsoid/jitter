# Command runner only: every target is phony. Cargo and Flutter own
# incremental builds; don't model file dependencies here.

.PHONY: check rust-check flutter-check gen fmt run cli appimage appimage-host

# The full gate, as CI runs it (scripts/check.sh). Run before committing.
check:
	scripts/check.sh

rust-check:
	scripts/check.sh rust

flutter-check:
	scripts/check.sh flutter

gen:
	rinf gen

# dart format has no exclude option; listing the files git sees skips
# the gitignored bindings.
fmt:
	cargo fmt
	git ls-files -z --cached --others --exclude-standard -- 'lib/*.dart' 'test/*.dart' | xargs -0 dart format

run: gen
	flutter run -d linux

# Pass CLI arguments with ARGS, e.g. `make cli ARGS="search foo"`.
cli:
	cargo run -p jitter_cli -- $(ARGS)

# Portable Linux build in dist/. Needs podman.
appimage:
	packaging/appimage/build.sh

# Linux build in dist/ from this machine's libraries; runs only where glibc
# is at least this machine's. Needs mpv.
appimage-host:
	packaging/appimage/host.sh
