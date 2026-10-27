import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:jitter/player/theatre.dart';

const _mediaKit = MethodChannel('com.alexmercerind/media_kit_video');

/// The system UI modes asked for, in order; the platform channel answers.
List<String> recordSystemUi(WidgetTester tester) {
  final modes = <String>[];
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
    SystemChannels.platform,
    (call) async {
      if (call.method == 'SystemChrome.setEnabledSystemUIMode') {
        modes.add(call.arguments as String);
      }
      return null;
    },
  );
  return modes;
}

/// Pumps a page with the theatre over a stand-in video, [on] at first;
/// the page turns it off on exit. Returns the number of exits.
Future<ValueNotifier<int>> pumpTheatre(
  WidgetTester tester, {
  bool on = true,
}) async {
  final exits = ValueNotifier(0);
  addTearDown(exits.dispose);
  await tester.pumpWidget(
    MaterialApp(
      home: _Host(on: on, exits: exits),
    ),
  );
  return exits;
}

class _Host extends StatefulWidget {
  const new({required this.on, required this.exits});

  final bool on;
  final ValueNotifier<int> exits;

  @override
  State<_Host> createState() => _HostState();
}

class _HostState extends State<_Host> {
  late bool on = widget.on;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Theatre(
        on: on,
        onExit: () => setState(() {
          on = false;
          widget.exits.value++;
        }),
        child: Column(
          children: [
            // Like media_kit's video, which takes Escape while focused.
            Focus(
              onKeyEvent: (_, event) =>
                  event.logicalKey == LogicalKeyboardKey.escape
                  ? KeyEventResult.handled
                  : KeyEventResult.ignored,
              child: const Text('Video'),
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

final Finder _exitButton = find.byTooltip('Exit theatre mode');

void main() {
  testWidgets('theatre hides the system bars and brings them back off', (
    tester,
  ) async {
    final modes = recordSystemUi(tester);
    await pumpTheatre(tester, on: false);
    expect(modes, isEmpty);

    await tester.tap(find.text('Toggle'));
    await tester.pump();
    await tester.tap(find.text('Toggle'));
    await tester.pump();

    expect(modes, ['SystemUiMode.immersiveSticky', 'SystemUiMode.edgeToEdge']);
  });

  testWidgets('leaving the page in theatre brings the system bars back', (
    tester,
  ) async {
    final modes = recordSystemUi(tester);
    await pumpTheatre(tester);

    await tester.pumpWidget(const SizedBox());

    expect(modes, ['SystemUiMode.immersiveSticky', 'SystemUiMode.edgeToEdge']);
  });

  testWidgets('on desktop theatre makes the window fullscreen', (tester) async {
    final calls = <String>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(_mediaKit, (
      call,
    ) async {
      calls.add(call.method);
      return null;
    });
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    await pumpTheatre(tester);

    await tester.tap(find.text('Toggle'));
    await tester.pump();
    debugDefaultTargetPlatformOverride = null;

    expect(calls, [
      'Utils.EnterNativeFullscreen',
      'Utils.ExitNativeFullscreen',
    ]);
  });

  testWidgets('the exit button shows on entering, fades, and comes back '
      'on a tap', (tester) async {
    recordSystemUi(tester);
    await pumpTheatre(tester);
    expect(_exitButton, findsOneWidget);

    await tester.pump(const Duration(seconds: 4));
    expect(_exitButton, findsNothing);

    await tester.tap(find.text('Video'));
    await tester.pump();
    expect(_exitButton, findsOneWidget);
  });

  testWidgets('the exit button comes first on Tab and stays while focused', (
    tester,
  ) async {
    recordSystemUi(tester);
    await pumpTheatre(tester);
    bool exitFocused() =>
        primaryFocus?.context?.findAncestorWidgetOfExactType<IconButton>() !=
        null;

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    expect(exitFocused(), isTrue);

    await tester.pump(const Duration(seconds: 4));
    expect(_exitButton, findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    expect(exitFocused(), isFalse);
    expect(primaryFocus?.context, isNotNull);
    expect(_exitButton, findsOneWidget);
    await tester.pump(const Duration(seconds: 4));
    expect(_exitButton, findsNothing);
  });

  testWidgets('the exit button leaves theatre', (tester) async {
    recordSystemUi(tester);
    final exits = await pumpTheatre(tester);

    await tester.tap(_exitButton);
    await tester.pump();

    expect(exits.value, 1);
    expect(_exitButton, findsNothing);
  });

  testWidgets('no exit button and no Escape out of theatre', (tester) async {
    recordSystemUi(tester);
    final exits = await pumpTheatre(tester, on: false);

    await tester.tap(find.text('Video'));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();

    expect(_exitButton, findsNothing);
    expect(exits.value, 0);
  });

  testWidgets('Escape leaves theatre, ahead of the focused widget', (
    tester,
  ) async {
    recordSystemUi(tester);
    final exits = await pumpTheatre(tester);
    Focus.of(tester.element(find.text('Video'))).requestFocus();
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();

    expect(exits.value, 1);
  });

  testWidgets("Escape on a route above the page is not theatre's", (
    tester,
  ) async {
    recordSystemUi(tester);
    final exits = await pumpTheatre(tester);
    Navigator.of(
      tester.element(find.text('Video')),
    ).push(MaterialPageRoute<void>(builder: (_) => const Text('Fullscreen')));
    await tester.pumpAndSettle();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);

    expect(exits.value, 0);
  });

  testWidgets('Back leaves theatre before it leaves the page', (tester) async {
    recordSystemUi(tester);
    await tester.pumpWidget(const MaterialApp(home: Text('Home')));
    final exits = ValueNotifier(0);
    addTearDown(exits.dispose);
    Navigator.of(tester.element(find.text('Home'))).push(
      MaterialPageRoute<void>(builder: (_) => _Host(on: true, exits: exits)),
    );
    await tester.pumpAndSettle();

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(exits.value, 1);
    expect(find.text('Video'), findsOneWidget);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.text('Video'), findsNothing);
  });
}
