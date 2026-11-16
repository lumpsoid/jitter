import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/fvp_engine_setup.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:logging/logging.dart';

void main() {
  group('picksFvp', () {
    test('JITTER_ENGINE=fvp picks fvp', () {
      expect(picksFvp(const {'JITTER_ENGINE': 'fvp'}), isTrue);
    });

    test('no variable keeps media_kit', () {
      expect(picksFvp(const {}), isFalse);
    });

    test('another engine name keeps media_kit', () {
      expect(picksFvp(const {'JITTER_ENGINE': 'media_kit'}), isFalse);
    });

    test('an empty JITTER_ENGINE keeps media_kit', () {
      expect(picksFvp(const {'JITTER_ENGINE': ''}), isFalse);
    });
  });

  group('rustLevelFor', () {
    test('shout is an error', () {
      expect(rustLevelFor(Level.SHOUT), DartLogLevel.error);
    });

    test('severe is an error', () {
      expect(rustLevelFor(Level.SEVERE), DartLogLevel.error);
    });

    test('warning is a warning', () {
      expect(rustLevelFor(Level.WARNING), DartLogLevel.warning);
    });

    test('everything below a warning is info, never verbose', () {
      for (final level in [
        Level.INFO,
        Level.CONFIG,
        Level.FINE,
        Level.FINEST,
      ]) {
        expect(rustLevelFor(level), DartLogLevel.info, reason: '$level');
      }
    });
  });
}
