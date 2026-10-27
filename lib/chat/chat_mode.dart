/// What the player page shows of the chat, as Rust last sent it.
enum ChatMode {
  /// No live channel plays: no panel and no toggle.
  off,

  /// A live channel plays and the user hid its chat: a toggle to show it.
  hidden,

  /// The panel is shown, with a toggle to hide it.
  shown,
}
