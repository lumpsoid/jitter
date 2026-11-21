import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/fullscreen.dart';
import 'package:material_ui/material_ui.dart';

const _mediaKit = MethodChannel('com.alexmercerind/media_kit_video');

const _immersive =
    'SystemChrome.setEnabledSystemUIMode SystemUiMode.immersiveSticky';
const _edgeToEdge =
    'SystemChrome.setEnabledSystemUIMode SystemUiMode.edgeToEdge';
const _landscape =
    'SystemChrome.setPreferredOrientations '
    '[DeviceOrientation.landscapeLeft, DeviceOrientation.landscapeRight]';
const _upright = 'SystemChrome.setPreferredOrientations []';

/// The SystemChrome calls asked for, in order, as `method arguments`; the
/// platform channel answers.
List<String> recordPlatform(WidgetTester tester) {
  final calls = <String>[];
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
    SystemChannels.platform,
    (call) async {
      if (call.method == 'SystemChrome.setEnabledSystemUIMode' ||
          call.method == 'SystemChrome.setPreferredOrientations') {
        calls.add('${call.method} ${call.arguments}');
      }
      return null;
    },
  );
  return calls;
}

/// Pumps a page with the fullscreen over a stand-in video, [on] at first;
/// the page turns it off on exit. Returns the number of exits.
Future<ValueNotifier<int>> pumpFullscreen(
  WidgetTester tester, {
  bool on = true,
  bool theatre = false,
}) async {
  final exits = ValueNotifier(0);
  addTearDown(exits.dispose);
  await tester.pumpWidget(
    MaterialApp(
      home: _Host(on: on, theatre: theatre, exits: exits),
    ),
  );
  return exits;
}

class _Host extends StatefulWidget {
  const new({required this.on, required this.theatre, required this.exits});

  final bool on;
  final bool theatre;
  final ValueNotifier<int> exits;

  @override
  State<_Host> createState() => _HostState();
}

class _HostState extends State<_Host> {
  late bool on = widget.on;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Fullscreen(
        on: on,
        theatre: widget.theatre,
        onExit: () => setState(() {
          on = false;
          widget.exits.value++;
        }),
        child: Column(
          children: [
            Builder(
              builder: (context) =>
                  Text(Fullscreen.isOn(context) ? 'Full' : 'Not full'),
            ),
            TextButton(
              onPressed: () => setState(() => on = !on),
              child: const Text('Toggle'),
            ),
          ],
        ),
      ),
    );
  }
}

void main() {
  testWidgets('mobile fullscreen hides the bars and locks landscape, and '
      'leaving it restores the app mode', (tester) async {
    final calls = recordPlatform(tester);
    await pumpFullscreen(tester, on: false);
    expect(calls, isEmpty);

    await tester.tap(find.text('Toggle'));
    await tester.pump();
    await tester.tap(find.text('Toggle'));
    await tester.pump();

    expect(calls, [_immersive, _landscape, _edgeToEdge, _upright]);
  });

  testWidgets('mobile fullscreen from theatre goes back into theatre, not '
      'out of it', (tester) async {
    final calls = recordPlatform(tester);
    await pumpFullscreen(tester, on: false, theatre: true);

    await tester.tap(find.text('Toggle'));
    await tester.pump();
    await tester.tap(find.text('Toggle'));
    await tester.pump();

    expect(calls, [_immersive, _landscape, _upright, _immersive]);
  });

  testWidgets('leaving the page in fullscreen restores the app mode', (
    tester,
  ) async {
    final calls = recordPlatform(tester);
    await pumpFullscreen(tester);

    await tester.pumpWidget(const SizedBox());
    await tester.pump();

    expect(calls, [_immersive, _landscape, _edgeToEdge, _upright]);
  });

  testWidgets('leaving the page in fullscreen from theatre restores the app '
      'mode, not theatre', (tester) async {
    final calls = recordPlatform(tester);
    await pumpFullscreen(tester, theatre: true);

    await tester.pumpWidget(const SizedBox());
    await tester.pump();

    expect(calls, [_immersive, _landscape, _edgeToEdge, _upright]);
  });

  testWidgets('on desktop fullscreen makes the window fullscreen', (
    tester,
  ) async {
    final calls = <String>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(_mediaKit, (
      call,
    ) async {
      calls.add(call.method);
      return null;
    });
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    await pumpFullscreen(tester, on: false);

    await tester.tap(find.text('Toggle'));
    await tester.pump();
    await tester.tap(find.text('Toggle'));
    await tester.pump();
    debugDefaultTargetPlatformOverride = null;

    expect(calls, [
      'Utils.EnterNativeFullscreen',
      'Utils.ExitNativeFullscreen',
    ]);
  });

  testWidgets('isOn reads the flag of the nearest fullscreen', (tester) async {
    recordPlatform(tester);
    await pumpFullscreen(tester, on: false);
    expect(find.text('Not full'), findsOneWidget);

    await tester.tap(find.text('Toggle'));
    await tester.pump();

    expect(find.text('Full'), findsOneWidget);
    expect(find.text('Not full'), findsNothing);
  });

  testWidgets('isOn is false with no fullscreen above', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) =>
              Text(Fullscreen.isOn(context) ? 'Full' : 'Not full'),
        ),
      ),
    );

    expect(find.text('Not full'), findsOneWidget);
  });

  testWidgets('Back leaves fullscreen before it leaves the page', (
    tester,
  ) async {
    recordPlatform(tester);
    await tester.pumpWidget(const MaterialApp(home: Text('Home')));
    final exits = ValueNotifier(0);
    addTearDown(exits.dispose);
    Navigator.of(tester.element(find.text('Home'))).push(
      MaterialPageRoute<void>(
        builder: (_) => _Host(on: true, theatre: false, exits: exits),
      ),
    );
    await tester.pumpAndSettle();

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(exits.value, 1);
    expect(find.text('Toggle'), findsOneWidget);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.text('Toggle'), findsNothing);
  });
}
