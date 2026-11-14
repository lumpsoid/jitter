/// What the bench plays. Parsed from the text the user types here; resolved
/// to a stream URL by `resolver.dart`. Pure Dart, so `flutter test` covers
/// it without the hub.
sealed class BenchTarget {
  const new();

  /// What the bench prints for this target. Never a full direct URL, which
  /// may carry a token.
  String get label;
}

/// A channel, played live.
final class LiveTarget extends BenchTarget {
  const new(this.login);

  final String login;

  @override
  String get label => 'live:$login';
}

/// A VOD, by its numeric id.
final class VodTarget extends BenchTarget {
  const new(this.id);

  final String id;

  @override
  String get label => 'vod:$id';
}

/// A direct stream URL, played as typed.
final class UrlTarget extends BenchTarget {
  const new(this.url);

  final String url;

  @override
  String get label => 'url:${Uri.tryParse(url)?.host ?? ''}';
}

const _webSchemes = {'http', 'https'};
const _twitchHosts = {'twitch.tv', 'www.twitch.tv', 'm.twitch.tv'};
final _separator = RegExp(r'[\s,]+');
final _digits = RegExp(r'^[0-9]+$');

/// How many times one target plays. The Windows device reset shows on the
/// second stream, so a single entry plays three times.
const _singleTargetRuns = 3;

/// The targets in [text], which holds entries separated by whitespace or
/// commas, in the order given. Blank text gives no targets.
///
/// A `twitch.tv` link (with `www.` or `m.`) with one path segment is a
/// live channel and `/videos/<digits>` is a VOD; its trailing `/` and query
/// are dropped. Any other `http(s)://` entry is a direct URL. A bare
/// number is a VOD id; anything else is a live login, lower-cased.
List<BenchTarget> parseTargets(String text) => [
  for (final entry in text.split(_separator))
    if (entry.isNotEmpty) _parseEntry(entry),
];

BenchTarget _parseEntry(String entry) {
  final uri = Uri.tryParse(entry);
  if (uri == null || !_webSchemes.contains(uri.scheme)) {
    if (_digits.hasMatch(entry)) return VodTarget(entry);
    return LiveTarget(entry.toLowerCase());
  }
  if (!_twitchHosts.contains(uri.host)) return UrlTarget(entry);
  final segments = uri.pathSegments.where((s) => s.isNotEmpty).toList();
  return switch (segments) {
    ['videos', final id] when _digits.hasMatch(id) => VodTarget(id),
    [final login] => LiveTarget(login.toLowerCase()),
    _ => UrlTarget(entry),
  };
}

/// The order the bench plays [targets] in. One target plays
/// [_singleTargetRuns] times in a row; two or more play once each, in the
/// order given.
List<BenchTarget> runOrder(List<BenchTarget> targets) {
  if (targets.length != 1) return List.of(targets);
  return [for (var run = 0; run < _singleTargetRuns; run++) targets.single];
}
