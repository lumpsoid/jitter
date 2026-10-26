import 'package:flutter/material.dart';
import 'package:jitter/src/bindings/bindings.dart';
import 'package:rinf/rinf.dart';

Future<void> main() async {
  await initializeRust(assignRustSignal);
  runApp(const JitterApp());
}

class JitterApp extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      title: 'Jitter',
      home: Scaffold(body: Center(child: Text('Jitter'))),
    );
  }
}
