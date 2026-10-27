# Native Android head

What a native Android GUI (Kotlin/Compose on top of the Rust crates) would
take, should we decide to build one. Not started; this records the analysis
so the decision does not have to be re-derived.

## Why it is tractable

- **A second GUI head already exists.** `native/gtk` drives core, twitch and
  store without rinf. Its `run.rs` loop is the same shape as
  `hub/src/run.rs`: owns the controllers, turns each intent into one
  controller action, shows the state after it. An Android head is a third
  instance of that pattern, not a new one.
- **The Rust stack already ships on Android.** The Flutter app builds `hub`
  as a cdylib for Android today, with rustls, bundled rusqlite and
  tokio-tungstenite. No new cross-compilation risk.
- **Platform inputs are already intents.** `Boot { dir }` carries the app's
  data directory; app backgrounded/foregrounded and the sleep timer are
  signals. Kotlin sends the same things.

## What it needs

1. **A crate `native/android`** (cdylib, UniFFI). Plays the hub's role:
   - one `send(intent)` entry point, with an `Intent` enum mirroring the
     hub's `DartSignal`s;
   - a callback interface Rust pushes view states through, mirroring the
     `RustSignal`s;
   - a tokio runtime started on first init.
2. **A build pipeline.** `cargo-ndk` for `arm64-v8a` (and `x86_64` for the
   emulator), `uniffi-bindgen` for the Kotlin bindings, a Gradle task running
   both, and a JNA dependency. Its own Gradle project: the root `android/`
   directory is Flutter's runner.
3. **The Kotlin/Compose UI** — the bulk of the work:
   - screens: browse, categories, channel, VODs, search, favorites,
     settings, chat;
   - player: Media3/ExoPlayer instead of media_kit, a `MediaSessionService`
     instead of audio_service for background playback;
   - the VOD controls redone: seek bar drag, double-tap seek, go-to-time,
     hold-to-show controls.
4. **Thread and lifecycle glue.** Callbacks arrive on tokio threads; push
   them into `StateFlow`s collected on the main thread. Rust state is
   process-wide and outlives Activity recreation, so a re-attaching Activity
   asks for the current state again, as `Booted` answers every `Boot`.

## Rough cost

| Part                                 | Effort            |
|--------------------------------------|-------------------|
| UniFFI crate + build pipeline        | a few days        |
| Media3 player + media session        | about a week      |
| Compose screens + chat               | several weeks     |

Gain: native look and feel, the platform player instead of mpv. Lose: one
GUI codebase — desktop still needs Flutter or GTK.

Start with a spike matching GTK's scope (browse top streams, play one) to
prove the UniFFI/Gradle pipeline before committing to the full UI.

## UniFFI, not hand-written JNI

**Hand-written JNI** (the `jni` crate) means writing every crossing by hand:
a `Java_app_jitter_Core_play(env, class, login)` function per call,
matched to a Kotlin `external fun` by naming convention with nothing
checking it. Strings and primitives cross easily; a struct does not — either
construct the Java object from Rust (find class, find constructor, call it)
or serialize to bytes and decode in Kotlin. Callbacks need the tokio thread
attached to the JVM and global refs to the listener; mistakes crash at
runtime, not at compile time.

**UniFFI** generates both sides from annotations:

```rust
#[derive(uniffi::Record)] pub struct StreamCard { pub login: String, pub viewers: String }
#[derive(uniffi::Enum)]   pub enum Intent { RefreshBrowse, Play { login: String } }
#[uniffi::export(callback_interface)] pub trait ViewSink { fn browse(&self, update: BrowseUpdate); }
#[uniffi::export] pub fn send(intent: Intent) { /* … */ }
```

Kotlin gets a `data class`, a `sealed class` and an `interface`, type-checked
against the Rust. Threads, refs and conversions are handled. Cost: a JNA
dependency, somewhat slower calls (fine for intents and view state, not for
per-frame work), a codegen step in the build. With ~80 message types
crossing, writing the conversions by hand is not worth it.

## Decide first: share the view mapping

Each hub feature module (e.g. `hub/src/browse.rs`) does three jobs:

1. **Drive** — call the controller (`browse.begin_refresh()`), spawn the
   Twitch fetch, route the result back (`on_page`). Loop glue.
2. **Map state to view** — `BrowseUpdate::shown`/`since` decide `Snapshot`
   vs `Appended`; building a `StreamCard` from a `Stream` picks the thumbnail
   size and the label. Pure: plain values in, plain values out.
3. **Send** — `.send_signal_to_dart()` and `#[derive(RustSignal)]` on the
   structs in `hub/src/signals/`. The only rinf-specific part.

Job 2, plus the labels in `hub/src/format.rs` (`viewers_label`,
`duration_label`, `uptime_label`, `streamed_label`, `twitch_error`, …), is
the rinf-free part. GTK already copies some of it: `gtk/src/format.rs`
re-implements `viewers_label`/`compact`/`tenths` ("worded like the Flutter
head's"), and `gtk/src/browse.rs` restates the append-within-one-refresh
rule. An Android head would be a third copy, and copies drift.

**Option: a shared `native/view` crate** holding the view structs, the
state-to-view mapping and the labels, with per-head derives behind
features — Rust cannot derive a trait on another crate's struct, so the
derives sit where the struct is defined:

```rust
#[cfg_attr(feature = "rinf",   derive(Serialize, rinf::RustSignal))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct StreamCard { /* … */ }
```

Each head then keeps only jobs 1 and 3, which are what actually differ
between heads.

With two heads, and GTK covering only browse and play, the duplication is
small. A third, full-coverage head is where it starts to cost. If the
Android head goes ahead, do this extraction first, as its own change.
