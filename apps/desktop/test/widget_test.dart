import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:networkhub_desktop/capability_panel.dart';
import 'package:networkhub_desktop/daemon_client.dart';
import 'package:networkhub_desktop/home_page.dart';
import 'package:networkhub_sdk/networkhub_sdk.dart';

Capability _cap(CapabilityStatus status, String reason) =>
    Capability(status: status, reason: reason);

void main() {
  testWidgets('CapabilityPanel renders keys, statuses, and reasons', (
    tester,
  ) async {
    final caps = {
      'vpn.wireguard': _cap(
        CapabilityStatus.CAPABILITY_STATUS_SUPPORTED,
        'userspace WireGuard',
      ),
      'hotspot.wifi_ap': _cap(
        CapabilityStatus.CAPABILITY_STATUS_UNSUPPORTED,
        'no programmatic hotspot API on macOS',
      ),
    };
    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: CapabilityPanel(capabilities: caps))),
    );

    expect(find.text('vpn.wireguard'), findsOneWidget);
    expect(find.text('hotspot.wifi_ap'), findsOneWidget);
    expect(find.text('supported'), findsOneWidget);
    expect(find.text('unsupported'), findsOneWidget);
  });

  testWidgets('HomePage renders the connected view from an injected snapshot', (
    tester,
  ) async {
    final snapshot = DaemonSnapshot(
      version: GetVersionResponse(
        daemonVersion: '0.0.0',
        apiVersion: 'v1',
        os: 'linux',
      ),
      capabilities: {
        'vpn.wireguard':
            _cap(CapabilityStatus.CAPABILITY_STATUS_SUPPORTED, 'ok'),
      },
    );
    await tester.pumpWidget(
      MaterialApp(home: HomePage(dir: '/tmp/x', fetch: (_) async => snapshot)),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('api v1'), findsOneWidget);
    expect(find.text('vpn.wireguard'), findsOneWidget);
  });

  testWidgets('HomePage shows the disconnected view on error', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: HomePage(
          dir: '/tmp/x',
          fetch: (_) async => throw Exception('connection refused'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Not connected to the daemon.'), findsOneWidget);
    expect(find.textContaining('connection refused'), findsOneWidget);
    expect(find.byType(FilledButton), findsOneWidget);
  });
}
