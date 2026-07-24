import 'package:flutter/material.dart';
import 'package:networkhub_sdk/networkhub_sdk.dart';

/// Renders the capability matrix (AD-6). The UI draws purely from this data —
/// there are no platform conditionals in feature code — so adding OS support is
/// a daemon change, never a UI change.
class CapabilityPanel extends StatelessWidget {
  const CapabilityPanel({required this.capabilities, super.key});

  /// Capability key → verdict, as reported by the daemon.
  final Map<String, Capability> capabilities;

  @override
  Widget build(BuildContext context) {
    final keys = capabilities.keys.toList()..sort();
    if (keys.isEmpty) {
      return const Center(child: Text('No capabilities reported.'));
    }
    return ListView.separated(
      itemCount: keys.length,
      separatorBuilder: (context, index) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final key = keys[index];
        final cap = capabilities[key]!;
        return ListTile(
          dense: true,
          leading: StatusChip(status: cap.status),
          title: Text(key),
          subtitle: Text(cap.reason),
        );
      },
    );
  }
}

/// A coloured chip conveying a capability's status.
class StatusChip extends StatelessWidget {
  const StatusChip({required this.status, super.key});

  /// The capability's status.
  final CapabilityStatus status;

  @override
  Widget build(BuildContext context) {
    final (label, color) = switch (status) {
      CapabilityStatus.CAPABILITY_STATUS_SUPPORTED => (
          'supported',
          Colors.green
        ),
      CapabilityStatus.CAPABILITY_STATUS_DEGRADED => (
          'degraded',
          Colors.orange
        ),
      CapabilityStatus.CAPABILITY_STATUS_UNSUPPORTED => (
          'unsupported',
          Colors.grey,
        ),
      CapabilityStatus.CAPABILITY_STATUS_UNKNOWN => (
          'unknown',
          Colors.blueGrey
        ),
      _ => ('unspecified', Colors.grey),
    };
    return Container(
      width: 104,
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        textAlign: TextAlign.center,
        style: TextStyle(color: color, fontWeight: FontWeight.w600),
      ),
    );
  }
}
