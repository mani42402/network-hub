import 'package:flutter/material.dart';

import 'capability_panel.dart';
import 'daemon_client.dart';

/// The main screen: connect to the daemon, check version compatibility, and
/// render its capability matrix. Reconnecting is a button press, so closing and
/// reopening the daemon is recoverable without restarting the app (§4.1 — the UI
/// is a view over daemon state, resynchronized on connect).
class HomePage extends StatefulWidget {
  const HomePage({required this.dir, this.fetch, super.key});

  /// The daemon directory to connect under.
  final String dir;

  /// How a snapshot is fetched. Injectable so tests need no real daemon;
  /// defaults to a real connection.
  final Future<DaemonSnapshot> Function(String dir)? fetch;

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  late Future<DaemonSnapshot> _future;

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() {
    final fetch = widget.fetch ?? fetchSnapshot;
    setState(() {
      _future = fetch(widget.dir);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('NetworkHub'),
        actions: [
          IconButton(
            onPressed: _load,
            icon: const Icon(Icons.refresh),
            tooltip: 'Reconnect',
          ),
        ],
      ),
      body: FutureBuilder<DaemonSnapshot>(
        future: _future,
        builder: (context, snap) {
          if (snap.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }
          if (snap.hasError) {
            return _DisconnectedView(
              dir: widget.dir,
              error: '${snap.error}',
              onRetry: _load,
            );
          }
          return _ConnectedView(snapshot: snap.data!);
        },
      ),
    );
  }
}

/// The connected state: a version header plus the capability matrix.
class _ConnectedView extends StatelessWidget {
  const _ConnectedView({required this.snapshot});

  final DaemonSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final v = snapshot.version;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.all(16),
          child: Text(
            'daemon ${v.daemonVersion} · api ${v.apiVersion} · ${v.os}',
            style: Theme.of(context).textTheme.titleMedium,
          ),
        ),
        const Divider(height: 1),
        Expanded(child: CapabilityPanel(capabilities: snapshot.capabilities)),
      ],
    );
  }
}

/// The disconnected state: an explanation and a retry button.
class _DisconnectedView extends StatelessWidget {
  const _DisconnectedView({
    required this.dir,
    required this.error,
    required this.onRetry,
  });

  final String dir;
  final String error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.cloud_off, size: 48),
            const SizedBox(height: 12),
            const Text('Not connected to the daemon.'),
            const SizedBox(height: 4),
            Text(
              'Looked under: $dir\n$error',
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: const Text('Reconnect'),
            ),
          ],
        ),
      ),
    );
  }
}
