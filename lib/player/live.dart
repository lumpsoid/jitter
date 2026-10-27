/// A value the player changes, such as where it plays: what it is [now],
/// and each change after. A widget mounted anew each time controls show
/// reads [now] for its first frame; a stream's first value would come a
/// frame late, and the frame before it would draw a zero.
class Live<T> {
  const new({required this.now, required this.changes});

  final T Function() now;

  /// Each call a new stream: each mount listens from then on.
  final Stream<T> Function() changes;
}
