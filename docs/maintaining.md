# Maintaining Jitter

The maintainer's jobs, in the order they come up.

## Fresh checkout

```sh
cargo install rinf_cli --version 8.11.0 --locked   # match the rinf crate
rinf gen                                             # bindings are gitignored
make check
```

For Android, `scripts/libmpv-android.sh fetch` gets the patched libmpv jars
(see [below](#patched-libmpv-for-android)), and put `android/key.properties`
back (see [Signing key](#android-signing-key)).

## Everyday

- `make check` before every commit; it is `scripts/check.sh`, the same gates
  CI runs.
- `make fmt` formats Rust and Dart.
- Add a line under `## [Unreleased]` in `CHANGELOG.md` for anything a user
  would notice.
- Pull requests: CI must be green, and each commit must pass the gates on
  its own. Rebase the branch on `main`, then land it with a merge commit
  (GitHub's "Create a merge commit"); no squash or fast-forward merges.

## Version

The version is in two files that must agree, and `scripts/version.sh check`
(part of `make check`) fails when they don't:

- `Cargo.toml`, `[workspace.package] version`
- `pubspec.yaml`, `version: X.Y.Z+N`, where `N` is Android's `versionCode`.
  It must grow with every release or Android refuses the update.

The app shows `X.Y.Z (commit)` under Settings → About and in
`jitter --version`; `native/core/build.rs` makes it.

Stay below 1.0 while things still change: a new feature bumps the minor
version, a fix the patch.

## Release

1. On `main`, clean and up to date, with the changes listed under
   `## [Unreleased]`.
2. `scripts/version.sh bump X.Y.Z`: sets both files, `N += 1`, renames
   `[Unreleased]` to `[X.Y.Z] - <today>` with a fresh `[Unreleased]` above
   it, fixes the compare links, refreshes `Cargo.lock`.
3. Review the diff, then `git commit -am "chore(release): X.Y.Z"`.
4. `scripts/release.sh`: checks everything, runs every gate, and makes the
   annotated tag `vX.Y.Z` with the changelog section as its message. It does
   not push.
5. `git push origin main vX.Y.Z`.

The tag runs `.github/workflows/release.yml`: it checks the tag against the
version, builds the signed APK, the AppImage and the Windows zip, and
publishes them with `SHA256SUMS` and the changelog section as the notes.
If it fails, fix it on `main`, delete the tag locally and on GitHub
(`git push origin :refs/tags/vX.Y.Z`), and release again; nothing was
published.

## Windows test builds

To check a fix on Windows before it ships, or to send it to the user who
reported the bug: Actions > Windows > Run workflow, pick the branch. The
same `.github/workflows/windows.yml` builds the release zip.

With "Publish the zip" ticked (the default), it also publishes the zip as
the prerelease `test-windows-<short commit>`, which anyone can download
without a GitHub account; the run's summary links it. The zip is named
`jitter-X.Y.Z-test.<short commit>-windows-x64.zip`. Without it, the zip is
only the run's artifact, which needs a GitHub login.

```sh
gh workflow run windows.yml --ref <branch>       # the same, from a shell
gh release delete test-windows-<commit> --cleanup-tag --yes   # once done
```

Delete a test prerelease once the fix has shipped. The workflow must be
on `main` for the Run workflow button to appear; the branch you run it on
must have it too.

## Android signing key

One key signs every release. **If it is lost, installed apps can never be
updated again**: users would have to uninstall, losing their favorites and
history.

Create it once, outside the repo:

```sh
keytool -genkeypair -v -keystore ~/keys/jitter-release.jks -alias jitter \
  -keyalg RSA -keysize 4096 -validity 36500
chmod 600 ~/keys/jitter-release.jks
```

Back up the `.jks` file and both passwords in a password manager.

For local release builds, `android/key.properties` (gitignored):

```properties
storeFile=/home/you/keys/jitter-release.jks
storePassword=...
keyAlias=jitter
keyPassword=...
```

Without it, a release build (`flutter build apk --release`, `flutter run
--release`) fails; debug and profile builds don't need the key. Check an
APK's signer with
`$ANDROID_HOME/build-tools/<version>/apksigner verify --print-certs <apk>`:
it must show your name, not `CN=Android Debug`.

CI reads the key from repository secrets; `gh secret set` prompts for each
value, so the passwords stay out of the shell history:

```sh
base64 -w0 ~/keys/jitter-release.jks | gh secret set ANDROID_KEYSTORE_BASE64 -R lumpsoid/jitter
gh secret set ANDROID_KEYSTORE_PASSWORD -R lumpsoid/jitter
gh secret set ANDROID_KEY_ALIAS -R lumpsoid/jitter
gh secret set ANDROID_KEY_PASSWORD -R lumpsoid/jitter
```

The release workflow stops early, naming the missing secret. A password
with `\`, `=` or `:` in it needs escaping in
`key.properties`; avoid them.

## Patched libmpv for Android

The Android build needs the patched libmpv jars in
`dist/libmpv-android/` (gitignored). `scripts/libmpv-android.sh` handles
them; the release workflow and contributors only ever `fetch`.

- `third_party/media_kit_libs_android_video/libmpv/jars.md5` pins them.
  Gradle refuses jars that don't match.
- They are stored as assets of a GitHub release named
  `libmpv-android-<hash of jars.md5>`. New jars get a new release, and old
  app versions keep theirs. `scripts/libmpv-android.sh name` prints it.

First time, with the jars you already have:

```sh
scripts/libmpv-android.sh verify    # they match the pins
scripts/libmpv-android.sh publish
```

After changing the patches or the upstream version:

```sh
scripts/libmpv-android.sh build     # an hour or more; writes jars.md5
scripts/libmpv-android.sh publish   # before pushing the new pins
git commit third_party/media_kit_libs_android_video/libmpv/jars.md5 -m "..."
```

Publish before pushing: a pushed pin with no release behind it fails the
next app release. `publish` does nothing if that release already exists.
The build's prerequisites are in the header of `libmpv/build.sh`.

## libmpv for Windows

The Windows build downloads shinchiro's prebuilt libmpv (current mpv and
FFmpeg, no patches needed) itself; `scripts/libmpv-windows.sh` only keeps
a copy for it. Why, and the license: the
[libmpv README](../third_party/media_kit_libs_windows_video/libmpv/README.md).

- `third_party/media_kit_libs_windows_video/libmpv/archive.md5` pins the
  archive. CMake refuses one that doesn't match.
- It is stored as an asset of a GitHub release named
  `libmpv-windows-<first 8 of its MD5>`, copied from shinchiro's
  SourceForge mirror, which keeps builds longer than shinchiro's GitHub
  releases. `scripts/libmpv-windows.sh name` prints it.

To update to a newer build:

```sh
scripts/libmpv-windows.sh latest            # the newest build on SourceForge
scripts/libmpv-windows.sh update [YYYYMMDD] # default: the newest; writes archive.md5
scripts/libmpv-windows.sh publish           # before pushing the new pin
git commit third_party/media_kit_libs_windows_video/libmpv/archive.md5 -m "..."
```

`update` checks the download against SourceForge's MD5 and that it has the
files the build needs. Then test the Windows build from the pushed branch:
live, a VOD seek outside the buffer, a VOD still recording, a quality switch.
As with Android, publish before pushing.

## Dependency updates

Dependabot opens weekly pull requests for Cargo, pub, Gradle and GitHub
Actions, minor and patch updates grouped. Merge them when CI is green.

These move together, by hand, in one commit:

- **rinf**: the crate in `native/hub/Cargo.toml`, the Dart package in
  `pubspec.yaml`, and `RINF_VERSION` in `.github/workflows/ci.yml`,
  `release.yml`, `windows.yml` and `packaging/appimage/Containerfile`.
  Then `rinf gen`
  and `make check`. Close Dependabot's pull request for one half and do
  it by hand.
- **Flutter and Rust toolchains**: `FLUTTER_VERSION` and `RUST_VERSION` in
  the three workflows and the Containerfile.
- **media_kit**: `third_party/` holds patched copies of
  `media_kit_video`, `media_kit_libs_android_video` and
  `media_kit_libs_windows_video`; an update means re-applying the patches
  (`third_party/media_kit_video/PATCHED.md`, the libmpv READMEs).

## One-time GitHub settings

- Description and topics: `twitch`, `flutter`, `rust`, `android`, `linux`,
  `steam-deck`, `mpv`.
- Settings → Code security: Dependabot alerts and security updates on.
- Settings → Actions → General: workflow permissions read-only (the release
  job asks for `contents: write` itself).
- Optional: a branch rule on `main` that requires the `Rust gates` and
  `Flutter gates` checks.
