import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/boot/boot_holder.dart';

void main() {
  test('starts pending', () {
    expect(BootHolder().status, isA<BootPending>());
  });

  test('Booted makes it ready and notifies', () {
    final holder = BootHolder();
    var notified = 0;
    holder
      ..addListener(() => notified++)
      ..booted();

    expect(holder.status, isA<BootReady>());
    expect(notified, 1);
  });

  test('BootFailed keeps its message', () {
    final holder = BootHolder()..fail('Storage failed: disk full');

    expect(
      holder.status,
      isA<BootError>().having(
        (s) => s.message,
        'message',
        'Storage failed: disk full',
      ),
    );
  });

  test('Booted after a failure clears it', () {
    final holder = BootHolder()
      ..fail('Storage failed: locked')
      ..booted();

    expect(holder.status, isA<BootReady>());
  });
}
