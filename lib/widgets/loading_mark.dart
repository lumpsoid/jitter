import 'package:material_ui/material_ui.dart';

/// A centred hourglass and "Loading…", for any loading state that can show
/// at startup.
///
/// Static on purpose: Flutter 3.50 hangs on Wayland Linux when frames animate
/// at startup (https://github.com/flutter/flutter/issues/196529), and the boot
/// screen and every home tab are built at once, so a spinner would animate
/// from the first frame. Use this in place of a `CircularProgressIndicator`
/// wherever a loading state can show at startup.
class LoadingMark extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = theme.colorScheme.onSurfaceVariant;
    return Semantics(
      liveRegion: true,
      label: 'Loading',
      child: ExcludeSemantics(
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.hourglass_empty, size: 32, color: color),
              const SizedBox(height: 8),
              Text(
                'Loading…',
                style: theme.textTheme.bodyMedium?.copyWith(color: color),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
