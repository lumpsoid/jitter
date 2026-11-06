import 'package:jitter/src/bindings/bindings.dart';

const qualities = [
  StartQualityChoice(quality: StartQuality.source, label: 'Source'),
  StartQualityChoice(quality: StartQuality.p1080, label: '1080p'),
  StartQualityChoice(quality: StartQuality.p720, label: '720p'),
  StartQualityChoice(quality: StartQuality.audioOnly, label: 'Audio only'),
];

SettingsUpdateSnapshot snapshot({
  AppTheme theme = AppTheme.system,
  bool animatedEmotes = true,
  bool autoLoadChat = true,
  StartQuality startQuality = StartQuality.source,
  bool thirdPartyEmotes = true,
  bool backgroundAudio = true,
  String version = '0.1.0 (2391c1d)',
}) => SettingsUpdateSnapshot(
  theme: theme,
  animatedEmotes: animatedEmotes,
  autoLoadChat: autoLoadChat,
  startQuality: startQuality,
  startQualities: qualities,
  thirdPartyEmotes: thirdPartyEmotes,
  backgroundAudio: backgroundAudio,
  version: version,
);
