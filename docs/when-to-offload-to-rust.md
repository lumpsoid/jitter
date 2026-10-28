# When to offload work to Rust

Rust owns state and decides; Dart renders. That rule is about **where
decisions live**, not about speed. This note is about the other question:
once Dart already holds data Rust sent, is it worth asking Rust to compute
something from it? The answer, measured, is usually no. A round trip over
the rinf bridge costs more than most of the work it would save.

## Measurements

Taken on 2026-10-28, on Linux x64. The cost being weighed: showing one
chatter's messages from a full chat buffer, either by filtering the
holder's entries in Dart or by asking Rust. 600 entries from 30 chatters,
so 20 matches. Each line is about 240 bytes encoded: an id, a sender, a
name, three fragments, one of them an emote URL.

| Step | Time |
|---|---|
| Dart: filter 600 entries by sender id (`flutter test`, JIT) | 1.5 µs |
| Rust: the same filter over `ChatController` entries (release) | 1.2 µs |
| Rust: map to signal types + `rinf::serialize`, 1 line | 0.8 µs |
| Rust: same, 20 lines | 15 µs |
| Rust: same, 100 lines | 69 µs |
| Rust: same, 600 lines | 418 µs |
| Dart: `bincodeDeserialize`, 1 line (246 B) | 5 µs |
| Dart: same, 20 lines (4.7 KB) | 17 µs |
| Dart: same, 100 lines (23.5 KB) | 78 µs |
| Dart: same, 600 lines (142 KB) | 490 µs |

Round trip in the running app (`flutter run --profile`: AOT Dart, release
Rust). Dart sends a ping asking for N lines, a Rust task answers with them,
and Dart times send to decoded reply. 300 runs per size, after 30 to warm up:

| Reply | Median | p95 | Min |
|---|---|---|---|
| empty | 14 µs | 25 µs | 11 µs |
| 1 line | 17 µs | 21 µs | 15 µs |
| 20 lines | 62 µs | 2.9 ms | 33 µs |
| 100 lines | 1.4 ms | 3.5 ms | 257 µs |
| 600 lines | 2.2 ms | 4.8 ms | 798 µs |

The round trips ran while the app was starting up, so the larger medians
and the p95s include contention with startup work on both sides. Read the
minimums as the cost of the bridge itself, and the medians as what a busy
app may see.

## What the numbers say

- **The hop alone (~14 µs) is about 10× a 600-entry Dart filter.** Simple
  work over data Dart already holds (filter, find, count, group by key)
  never pays for a round trip.
- **Rust is not much faster at small work.** Comparing short strings runs
  at memory speed in either language: 1.2 µs against 1.5 µs.
- **The payload costs more than the hop.** Both ends pay about 0.7 µs per
  chat line, to serialize in Rust and to decode in Dart. Past about 100
  lines a reply costs more than a frame's slack, so it is a batching
  problem (see the chat batching in [rinf bridge](./rinf-bridge.md)).
- **A reply is a copy.** Rust sends fresh values, and Dart cannot tell they
  are the entries it already holds. Any later change to those entries
  (deleted, evicted, recoloured) needs its own update stream to reach
  whoever holds the copy. Work done in Dart over the holder's own objects
  stays current for free.

## The rule

Ask Rust when the answer needs **a decision or data Dart does not have**:

- a business rule (what counts, what is allowed, what comes next);
- state only Rust holds (the store, a controller's internals, the network);
- work far heavier than the round trip, on data Rust has anyway (parsing,
  tokenizing, sorting thousands of items).

Do it in Dart when it is **mechanical over view state Rust already sent**:
matching by an id or key Rust put there, filtering on such a key, or
counting, as the holders already do when they fold diffs by id.

The boundary between the two is "who decides what the key means". Rust
should send the key (a sender id, a message id) and decide what makes two
items the same. Dart only compares keys. Once the selection grows a rule,
for example "this user's messages *and the replies to them*", the rule
moves into core and Rust sends the result, whatever it costs.

## Reproducing

The bench code was temporary and is not in the tree. To run it again:

- **Dart side:** a `flutter test` file builds a `ChatHolder` from a 600-line
  `ChatUpdateSnapshot`, times `messagesFrom` with a `Stopwatch`, then times
  `ChatUpdate.bincodeDeserialize` on `ChatUpdateAppended(...)
  .bincodeSerialize()` bytes for each size.
- **Rust side:** a `#[test]` in `native/hub/src/chat.rs` fills a
  `ChatController` and times the filter and `item(...)` +
  `rinf::serialize(&ChatUpdate::Appended { .. })`. Run it with
  `cargo test --release -p hub <name> -- --nocapture`.
- **Round trip:** a `BenchPing { lines }` DartSignal, and a `BenchPong {
  messages: Vec<ChatItem> }` RustSignal answered by a task spawned in
  `hub::main`. Dart reads replies with a `StreamIterator` over
  `BenchPong.rustSignalStream` from `main()` after `initializeRust`. Run
  `rinf gen`, then `flutter run --profile -d linux`.
