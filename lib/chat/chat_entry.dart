import 'package:jitter/src/bindings/bindings.dart';

/// One message in the chat holder: the line as Rust sent it, and whether a
/// later `Marked` deleted it. Mutable, so a view holding on to entries (the
/// paused list) still sees a deletion.
class ChatEntry {
  new(this.line) : deleted = line.deleted;

  final ChatLine line;
  bool deleted;
}
