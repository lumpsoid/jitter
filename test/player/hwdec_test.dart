// The gamescope test goes with the gamescope case in `hwdecFor`:
// see docs/gamescope-vulkan-hwdec.md.

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

  test('JITTER_HWDEC wins, even under gamescope', () {
    expect(
      hwdecFor(const {
        'JITTER_HWDEC': 'd3d11va-copy',
        'GAMESCOPE_WAYLAND_DISPLAY': 'gamescope-0',
      }),
      'd3d11va-copy',
    );
  });

  test('an empty JITTER_HWDEC is ignored', () {
    expect(hwdecFor(const {'JITTER_HWDEC': ''}), isNull);
  });
}
