import 'package:flutter/material.dart';

/// Scrolled this close to the end, the view asks for the next page.
const double _nextPageExtent = 800;

/// [slivers] refreshed by pulling down and paged by scrolling: near the end
/// it calls [onNextPage].
class PagedScrollView extends StatelessWidget {
  const new({
    required this.onRefresh,
    required this.onNextPage,
    required this.slivers,
    super.key,
  });

  final RefreshCallback onRefresh;
  final VoidCallback onNextPage;
  final List<Widget> slivers;

  bool _onScroll(ScrollNotification notification) {
    if (notification.metrics.extentAfter < _nextPageExtent) onNextPage();
    return false;
  }

  @override
  Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: onRefresh,
      child: NotificationListener<ScrollNotification>(
        onNotification: _onScroll,
        child: CustomScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          slivers: slivers,
        ),
      ),
    );
  }
}
