import 'package:flutter/material.dart';

import 'daemon_client.dart';
import 'home_page.dart';

void main() {
  runApp(const NetworkHubApp());
}

/// The NetworkHub desktop application — an unprivileged gRPC client of the
/// daemon. Develop against `daemon-sim` so it needs no root (WP-M0.11/§14.1).
class NetworkHubApp extends StatelessWidget {
  const NetworkHubApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'NetworkHub',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true),
      home: HomePage(dir: defaultSimDir()),
    );
  }
}
