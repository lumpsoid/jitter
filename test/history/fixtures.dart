import 'package:jitter/src/bindings/bindings.dart';

HistoryEntry entry(
  int id, {
  String title = 'GTA RP',
  String login = 'xqc',
  String? vod,
  String when = '14:02',
  String? streamed,
  String? game = 'Grand Theft Auto V',
}) => HistoryEntry(
  id: id,
  target: vod == null
      ? PlaybackTargetLive(login: login)
      : PlaybackTargetVod(id: vod),
  live: vod == null,
  title: title,
  channel: PlaybackChannel(login: login, displayName: login.toUpperCase()),
  game: game,
  when: when,
  watched: '40m',
  streamed: streamed,
);

HistoryDay day(String label, List<HistoryEntry> entries) =>
    HistoryDay(label: label, entries: entries);

HistoryUpdateSnapshot snapshot({
  List<HistoryDay> days = const [],
  List<HistoryEntry> unique = const [],
}) => HistoryUpdateSnapshot(days: days, unique: unique);
