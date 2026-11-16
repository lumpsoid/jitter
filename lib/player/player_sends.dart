import 'package:jitter/src/bindings/bindings.dart';

/// The player page's way out to Rust: one sender per signal the page sends.
/// The defaults send to Rust; tests record with [PlayerSends.to].
class PlayerSends {
  const new({
    this.play = _play,
    this.playerPosition = _playerPosition,
    this.refreshPlayback = _refreshPlayback,
    this.selectQuality = _selectQuality,
    this.goToTime = _goToTime,
    this.goToChapter = _goToChapter,
    this.rewindStream = _rewindStream,
    this.backToLive = _backToLive,
    this.armSleepTimer = _armSleepTimer,
    this.turnOffSleepTimer = _turnOffSleepTimer,
    this.leavePlayer = _leavePlayer,
  });

  /// Every signal goes to [send]; tests without Rust record them there.
  const new to(void Function(Object signal) send)
    : play = send,
      playerPosition = send,
      refreshPlayback = send,
      selectQuality = send,
      goToTime = send,
      goToChapter = send,
      rewindStream = send,
      backToLive = send,
      armSleepTimer = send,
      turnOffSleepTimer = send,
      leavePlayer = send;

  final void Function(Play) play;
  final void Function(PlayerPosition) playerPosition;
  final void Function(RefreshPlayback) refreshPlayback;
  final void Function(SelectQuality) selectQuality;
  final void Function(GoToTime) goToTime;
  final void Function(GoToChapter) goToChapter;
  final void Function(RewindStream) rewindStream;
  final void Function(BackToLive) backToLive;
  final void Function(ArmSleepTimer) armSleepTimer;
  final void Function(TurnOffSleepTimer) turnOffSleepTimer;
  final void Function(LeavePlayer) leavePlayer;
}

void _play(Play signal) => signal.sendSignalToRust();

void _playerPosition(PlayerPosition signal) => signal.sendSignalToRust();

void _refreshPlayback(RefreshPlayback signal) => signal.sendSignalToRust();

void _selectQuality(SelectQuality signal) => signal.sendSignalToRust();

void _goToTime(GoToTime signal) => signal.sendSignalToRust();

void _goToChapter(GoToChapter signal) => signal.sendSignalToRust();

void _rewindStream(RewindStream signal) => signal.sendSignalToRust();

void _backToLive(BackToLive signal) => signal.sendSignalToRust();

void _armSleepTimer(ArmSleepTimer signal) => signal.sendSignalToRust();

void _turnOffSleepTimer(TurnOffSleepTimer signal) => signal.sendSignalToRust();

void _leavePlayer(LeavePlayer signal) => signal.sendSignalToRust();
