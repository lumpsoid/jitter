import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/media_kit_engine.dart';

void main() {
  test('under gamescope, decodes with VA-API only', () {
    expect(
      hwdecFor(const {'GAMESCOPE_WAYLAND_DISPLAY': 'gamescope-0'}),
      'vaapi,vaapi-copy',
    );
  });

  test("elsewhere, keeps media_kit's default", () {
    expect(hwdecFor(const {'WAYLAND_DISPLAY': 'wayland-0'}), isNull);
  });
}
