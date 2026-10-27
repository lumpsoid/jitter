import 'dart:ui' show AppExitResponse;

import 'package:flutter/material.dart';
import 'package:jitter/boot/boot_failure_page.dart';
import 'package:jitter/boot/boot_holder.dart';
import 'package:jitter/shell/app_shell.dart';
import 'package:jitter/top/browse_holder.dart';
import 'package:rinf/rinf.dart';

/// The app: a spinner until Rust has booted, its failure with a Retry, or
/// the navigation shell. Feature holders are created in `main` and passed
/// down from here.
class JitterApp extends StatefulWidget {
  const new({
    required this.boot,
    // Browse
    required this.browse,
    super.key,
  });

  final BootHolder boot;
  // Browse
  final BrowseHolder browse;

  @override
  State<JitterApp> createState() => _JitterAppState();
}

class _JitterAppState extends State<JitterApp> {
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    // Lets Rust shut down and close the database before the process ends.
    _lifecycle = AppLifecycleListener(
      onExitRequested: () async {
        finalizeRust();
        return AppExitResponse.exit;
      },
    );
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Jitter',
      theme: ThemeData(colorSchemeSeed: Colors.deepPurple),
      darkTheme: ThemeData(
        colorSchemeSeed: Colors.deepPurple,
        brightness: Brightness.dark,
      ),
      home: ListenableBuilder(
        listenable: widget.boot,
        builder: (context, _) => switch (widget.boot.status) {
          BootPending() => const Scaffold(
            body: Center(child: CircularProgressIndicator()),
          ),
          BootError(:final message) => BootFailurePage(
            message: message,
            onRetry: widget.boot.boot,
          ),
          BootReady() => AppShell(
            // Browse
            browse: widget.browse,
          ),
        },
      ),
    );
  }
}
