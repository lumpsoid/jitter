import 'dart:ui';

import 'package:jitter/src/bindings/bindings.dart';

/// Sends the screens [dispatcher] knows, and which has the app's window,
/// to this run's log.
void sendDisplaysProbe(PlatformDispatcher dispatcher) {
  final window = _windowDisplayId(dispatcher);
  DisplaysProbe(
    displays: [
      for (final display in dispatcher.displays)
        DisplayFacts(
          width: display.size.width,
          height: display.size.height,
          refreshHz: display.refreshRate,
          scale: display.devicePixelRatio,
          hasWindow: display.id == window,
        ),
    ],
  ).sendSignalToRust();
}

/// The screen the app's window is on; `null` where the embedder does not
/// say, as `FlutterView.display` then throws.
int? _windowDisplayId(PlatformDispatcher dispatcher) {
  try {
    return dispatcher.implicitView?.display.id;
  } on Object catch (_) {
    return null;
  }
}
