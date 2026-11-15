import 'package:flutter_test/flutter_test.dart';

import '../../bench/targets.dart';

List<String> _labels(List<BenchTarget> targets) => [
  for (final target in targets) target.label,
];

void main() {
  group('parseTargets', () {
    test('a bare word is a live login, lower-cased', () {
      expect(
        parseTargets('Shroud').single,
        isA<LiveTarget>().having((t) => t.login, 'login', 'shroud'),
      );
    });

    test('a bare number is a VOD id', () {
      expect(
        parseTargets('2312345678').single,
        isA<VodTarget>().having((t) => t.id, 'id', '2312345678'),
      );
    });

    test('a videos link is a VOD, and its query is dropped', () {
      expect(
        parseTargets('https://www.twitch.tv/videos/2312345678?t=1h').single,
        isA<VodTarget>().having((t) => t.id, 'id', '2312345678'),
      );
    });

    test('a channel link keeps only the login, without the trailing slash', () {
      expect(
        parseTargets('https://twitch.tv/xqc/').single,
        isA<LiveTarget>().having((t) => t.login, 'login', 'xqc'),
      );
    });

    test('the m. host is a channel link too', () {
      expect(
        parseTargets('https://m.twitch.tv/Xqc').single,
        isA<LiveTarget>().having((t) => t.login, 'login', 'xqc'),
      );
    });

    test('a direct URL keeps its full text, and its label is the host', () {
      final target = parseTargets('https://example.com/a.m3u8?token=x').single;
      expect(
        target,
        isA<UrlTarget>().having(
          (t) => t.url,
          'url',
          'https://example.com/a.m3u8?token=x',
        ),
      );
      expect(target.label, 'url:example.com');
    });

    test('commas and spaces separate entries, in order', () {
      expect(_labels(parseTargets('a, b  c')), ['live:a', 'live:b', 'live:c']);
    });

    test('blank text gives no targets', () {
      expect(parseTargets('  '), isEmpty);
    });
  });

  group('runOrder', () {
    test('one target plays three times in a row', () {
      expect(_labels(runOrder([const LiveTarget('a')])), [
        'live:a',
        'live:a',
        'live:a',
      ]);
    });

    test('two targets play once each, in the given order', () {
      expect(_labels(runOrder(parseTargets('a b'))), ['live:a', 'live:b']);
    });
  });
}
