import 'dart:io';

import 'package:grpc/grpc.dart';
import 'package:networkhub_sdk/networkhub_sdk.dart';

/// A point-in-time view of the daemon: its version and capability matrix.
class DaemonSnapshot {
  const DaemonSnapshot({required this.version, required this.capabilities});

  /// Version and API-contract information (ADR-0013).
  final GetVersionResponse version;

  /// The capability matrix the UI renders from (AD-6).
  final Map<String, Capability> capabilities;
}

/// A gRPC client for the local daemon over its Unix domain socket — the same
/// transport `daemon-sim` (WP-M0.11) serves. The Windows named-pipe transport
/// is a follow-up; on Unix this is the whole client.
class DaemonClient {
  DaemonClient._(this._channel, this.system);

  /// Connects to the daemon whose socket lives under [dir].
  factory DaemonClient.connect(String dir) {
    final path = '$dir${Platform.pathSeparator}daemon.sock';
    final channel = ClientChannel(
      InternetAddress(path, type: InternetAddressType.unix),
      port: 0,
      options: const ChannelOptions(credentials: ChannelCredentials.insecure()),
    );
    return DaemonClient._(channel, SystemServiceClient(channel));
  }

  final ClientChannel _channel;

  /// The system service: version, health, and capabilities.
  final SystemServiceClient system;

  /// Closes the connection.
  Future<void> shutdown() => _channel.shutdown();
}

/// The default `daemon-sim` directory, mirroring the `daemon-sim` binary.
String defaultSimDir() =>
    '${Directory.systemTemp.path}${Platform.pathSeparator}networkhub-sim';

/// Connects, reads the version and capabilities, and disconnects.
///
/// Throws if the daemon is unreachable; the UI turns that into a reconnect
/// prompt rather than a crash.
Future<DaemonSnapshot> fetchSnapshot(String dir) async {
  final client = DaemonClient.connect(dir);
  try {
    final version = await client.system.getVersion(GetVersionRequest());
    final caps = await client.system.getCapabilities(GetCapabilitiesRequest());
    return DaemonSnapshot(
      version: version,
      capabilities: Map<String, Capability>.from(caps.capabilities),
    );
  } finally {
    await client.shutdown();
  }
}
