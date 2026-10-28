import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/widgets/steam_keyboard.dart';

const _steamOs = '''
NAME="SteamOS"
PRETTY_NAME="SteamOS"
ID=steamos
ID_LIKE=arch
''';

const _arch = '''
NAME="Arch Linux"
ID=arch
''';

void main() {
  test('launched by Steam on a Deck, on SteamOS or in Game Mode', () {
    for (final name in ['SteamDeck', 'SteamOS', 'SteamGamepadUI']) {
      expect(
        steamKeyboardAvailable(environment: {name: '1'}, osRelease: _arch),
        isTrue,
        reason: name,
      );
    }
  });

  test('on SteamOS by its os-release, quoted or not', () {
    expect(
      steamKeyboardAvailable(environment: const {}, osRelease: _steamOs),
      isTrue,
    );
    expect(
      steamKeyboardAvailable(
        environment: const {},
        osRelease: 'ID="steamos"\n',
      ),
      isTrue,
    );
  });

  test('not elsewhere: another distribution, ID_LIKE alone, or no file', () {
    expect(
      steamKeyboardAvailable(environment: const {}, osRelease: _arch),
      isFalse,
    );
    expect(
      steamKeyboardAvailable(
        environment: const {},
        osRelease: 'ID=bazzite\nID_LIKE="steamos fedora"\n',
      ),
      isFalse,
    );
    expect(
      steamKeyboardAvailable(
        environment: const {'SteamDeck': '0'},
        osRelease: null,
      ),
      isFalse,
    );
  });
}
