import 'package:flutter/foundation.dart';
import 'package:jitter/player/player_engine.dart';
import 'package:video_player/video_player.dart';

/// The video_player (fvp, libmdk) engine for the load comparison with
/// `MediaKitEngine`. Dev-only: nothing in `lib/` may import it.
class FvpEngine implements PlayerEngine {
  final ValueNotifier<VideoPlayerController?> _controller =
      ValueNotifier<VideoPlayerController?>(null);
  double _level = 1;

  /// The playing controller, for the bench's picture.
  ValueListenable<VideoPlayerController?> get controller => _controller;

  @override
  Future<void> open(String url, {required Duration start}) async {
    await _release();
    final next = VideoPlayerController.networkUrl(Uri.parse(url));
    try {
      await next.initialize();
    } on Object {
      await next.dispose();
      rethrow;
    }
    if (start > Duration.zero) {
      await next.seekTo(start);
    }
    // The bench has no user volume, so the user's volume is 1.0.
    await next.setVolume(_level);
    await next.play();
    _controller.value = next;
  }

  @override
  Future<void> stop() async {
    _level = 1;
    await _release();
  }

  @override
  Future<void> setFade(double level) async {
    _level = level;
    await _controller.value?.setVolume(level);
  }

  @override
  Future<void> seek(Duration to) async {
    await _controller.value?.seekTo(to);
  }

  @override
  Duration get position => _controller.value?.value.position ?? Duration.zero;

  @override
  Duration get duration => _controller.value?.value.duration ?? Duration.zero;

  Future<void> _release() async {
    final previous = _controller.value;
    _controller.value = null;
    await previous?.dispose();
  }
}
