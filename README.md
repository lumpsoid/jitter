# jitter

A new Flutter project.

Jitter is not affiliated with, endorsed by, or sponsored by Twitch
Interactive, Inc. "Twitch" is a trademark of Twitch Interactive, Inc.

## Getting Started

This project is a starting point for a Flutter application.

A few resources to get you started if this is your first Flutter project:

- [Learn Flutter](https://docs.flutter.dev/get-started/learn-flutter)
- [Write your first Flutter app](https://docs.flutter.dev/get-started/codelab)
- [Flutter learning resources](https://docs.flutter.dev/reference/learning-resources)

For help getting started with Flutter development, view the
[online documentation](https://docs.flutter.dev/), which offers tutorials,
samples, guidance on mobile development, and a full API reference.

## Using Rust Inside Flutter

This project leverages Flutter for GUI and Rust for the backend logic,
utilizing the capabilities of the
[Rinf](https://pub.dev/packages/rinf) framework.

To run and build this app, you need to have
[Flutter SDK](https://docs.flutter.dev/get-started/install)
and [Rust toolchain](https://www.rust-lang.org/tools/install)
installed on your system.
You can check that your system is ready with the commands below.
Note that all the Flutter subcomponents should be installed.

```shell
rustc --version
flutter doctor
```

You also need to have the CLI tool for Rinf ready.

```shell
cargo install rinf_cli
```

Signals sent between Dart and Rust are implemented using signal attributes.
If you've modified the signal structs, run the following command
to generate the corresponding Dart classes:

```shell
rinf gen
```

Now you can run and build this app just like any other Flutter projects.

```shell
flutter run
```

For detailed instructions on writing Rust and Flutter together,
please refer to Rinf's [documentation](https://rinf.cunarist.org).

## Portable Linux build

`make appimage` builds a self-contained AppImage in `dist/`. Building it and
running it elsewhere, SteamOS Game Mode included, is covered in
[Linux AppImage](./docs/linux-appimage.md).

## License

Jitter's own code is under the [0BSD license](./LICENSE). It does not cover:

- `third_party/`: vendored, patched copies of media_kit packages, each under
  its own `LICENSE` (MIT). The exception is the `*.patch` files in
  [`third_party/media_kit_libs_android_video/libmpv/`](./third_party/media_kit_libs_android_video/libmpv/):
  those are upstream FFmpeg commits backported to FFmpeg 6.0. They keep their
  original authors and stay under FFmpeg's license.
- libmpv and FFmpeg. They are not in this repository, but release builds
  bundle them: the AppImage carries Ubuntu 24.04's packages, and the Android
  build carries media-kit's libmpv-android-video-build rebuilt with the
  patches above by that directory's `build.sh`. They stay under their own
  licenses (LGPL or GPL, depending on the build), and their source is
  available from their upstream projects, plus those patches.
