// SPIKE-A client: can Dart speak gRPC over a transport that is not TCP?
//
// ADR-0002 commits NetworkHub to gRPC over Unix domain sockets and Windows
// named pipes, authenticated by OS peer credentials. The Rust side of that is
// well-trodden. The Dart side is the risk, because `dart:io` has no named-pipe
// support and its AF_UNIX support on Windows is unverified.
//
// This program measures three things:
//   1. Does a unary call work over the transport?
//   2. Does server streaming work, which the live metrics and event feeds need?
//   3. What are the latency and throughput characteristics?
//
// Throwaway code. Deleted once ADR-0016 records the outcome.

import 'dart:async';
import 'dart:io';

import 'package:grpc/grpc.dart';

import '../lib/generated/spike.pbgrpc.dart';
import '../lib/named_pipe_windows.dart' show namedPipeChannel;

const socketPath = '/tmp/networkhub-spike-a.sock';
const pipeName = r'\\.\pipe\networkhub-spike-a';

Future<int> main(List<String> args) async {
  final transport = Platform.isWindows ? 'named-pipe' : 'uds';
  stdout.writeln('SPIKE-A Dart client');
  stdout.writeln('  platform:  ${Platform.operatingSystem} '
      '(${Platform.operatingSystemVersion})');
  stdout.writeln('  dart:      ${Platform.version.split(' ').first}');
  stdout.writeln('  transport: $transport');
  stdout.writeln('');

  if (Platform.isWindows) {
    if (args.contains('--try-afunix')) return await _tryAfUnixOnWindows();
    return await _runWindows();
  }
  return await _runUnix();
}

// ---------------------------------------------------------------------------
// Unix: Linux and macOS. dart:io supports AF_UNIX natively here, and
// grpc-dart's ClientChannel accepts an InternetAddress, so this should work
// without any FFI.
// ---------------------------------------------------------------------------
Future<int> _runUnix() async {
  if (!File(socketPath).existsSync()) {
    stderr.writeln('FAIL: no socket at $socketPath — is the server running?');
    return 1;
  }

  final channel = ClientChannel(
    InternetAddress(socketPath, type: InternetAddressType.unix),
    // Ignored for a Unix socket, but the API requires a value.
    port: 0,
    options: const ChannelOptions(credentials: ChannelCredentials.insecure()),
  );

  try {
    return await _exercise(SpikeServiceClient(channel));
  } finally {
    await channel.shutdown();
  }
}

// ---------------------------------------------------------------------------
// Windows. Two candidate paths, tried in order:
//
//   A1  AF_UNIX via dart:io. Windows 10 1803+ supports AF_UNIX at the OS level,
//       but Dart may not expose it. If this works, Windows is no different from
//       Unix and ADR-0002 stands unchanged.
//   A2  Named pipe via dart:ffi. Rust and tokio do not support AF_UNIX on
//       Windows anyway, so this is the likely answer: a small FFI shim wrapping
//       CreateFile/ReadFile/WriteFile, presented to grpc-dart as a socket-like
//       stream.
//
// Only A1 is attempted here. If it fails, the failure text is the finding, and
// A2 is implemented against the named-pipe server in the Rust binary.
// ---------------------------------------------------------------------------
Future<int> _runWindows() async {
  // A2: named pipe via FFI. Forced with --pipe, and also the default because
  // the Rust/tokio server has no AF_UNIX support on Windows to talk to. A1 is
  // kept as an explicit experiment (--try-afunix) to record whether Dart *can*
  // do AF_UNIX on Windows, independent of whether our server offers it.
  stdout.writeln('Using A2: named pipe via dart:ffi ($pipeName)');
  try {
    final channel = namedPipeChannel(pipeName);
    final code = await _exercise(SpikeServiceClient(channel));
    await channel.shutdown();
    stdout.writeln('\nRESULT: A2 works — named-pipe FFI transport carries gRPC.');
    return code;
  } catch (e, st) {
    stdout.writeln('A2 failed: $e');
    stdout.writeln(st);
    stdout.writeln('');
    stdout.writeln('RESULT: named-pipe transport did not work. This is the');
    stdout.writeln('finding — capture the error above in ADR-0016.');
    return 2;
  }
}

/// Records whether Dart can open an AF_UNIX socket on Windows at all. Not the
/// shipping path (the server is a named pipe), but a useful data point.
Future<int> _tryAfUnixOnWindows() async {
  stdout.writeln('Experiment A1: AF_UNIX via dart:io on Windows...');
  try {
    final channel = ClientChannel(
      InternetAddress(socketPath, type: InternetAddressType.unix),
      port: 0,
      options: const ChannelOptions(credentials: ChannelCredentials.insecure()),
    );
    final code = await _exercise(SpikeServiceClient(channel));
    await channel.shutdown();
    stdout.writeln('\nFINDING: Dart CAN open AF_UNIX on Windows.');
    return code;
  } catch (e) {
    stdout.writeln('\nFINDING: Dart cannot open AF_UNIX on Windows: $e');
    return 2;
  }
}

// ---------------------------------------------------------------------------

Future<int> _exercise(SpikeServiceClient client) async {
  var failures = 0;

  // --- 1. Unary + peer credentials ---------------------------------------
  stdout.writeln('[1/3] unary echo with peer credentials');
  try {
    final reply = await client.echo(EchoRequest()..message = 'hello');
    stdout.writeln('  reply:  ${reply.message}');
    stdout.writeln('  peer:   uid=${reply.peer.uid} gid=${reply.peer.gid} '
        'pid=${reply.peer.pid}');
    stdout.writeln('  via:    ${reply.peer.transport}');
    if (reply.peer.notes.isNotEmpty) {
      stdout.writeln('  notes:  ${reply.peer.notes}');
    }

    // The whole point of this transport is that the OS tells the server who is
    // calling. A uid of 0 from a non-root client would mean it did not.
    final expectedUid = _currentUid();
    if (expectedUid != null && reply.peer.uid != expectedUid) {
      stdout.writeln('  FAIL:   uid mismatch — expected $expectedUid');
      failures++;
    } else {
      stdout.writeln('  OK:     server identified the caller from the OS');
    }
  } catch (e) {
    stdout.writeln('  FAIL: $e');
    failures++;
  }

  // --- 2. Unary latency ---------------------------------------------------
  stdout.writeln('');
  stdout.writeln('[2/3] unary latency (200 calls)');
  try {
    final samples = <int>[];
    for (var i = 0; i < 200; i++) {
      final sw = Stopwatch()..start();
      await client.echo(EchoRequest()..message = 'ping-$i');
      sw.stop();
      samples.add(sw.elapsedMicroseconds);
    }
    samples.sort();
    stdout.writeln('  p50:  ${_fmtMicros(samples[samples.length ~/ 2])}');
    stdout.writeln('  p95:  ${_fmtMicros(samples[(samples.length * 95) ~/ 100])}');
    stdout.writeln('  p99:  ${_fmtMicros(samples[(samples.length * 99) ~/ 100])}');
    stdout.writeln('  max:  ${_fmtMicros(samples.last)}');
  } catch (e) {
    stdout.writeln('  FAIL: $e');
    failures++;
  }

  // --- 3. Server streaming ------------------------------------------------
  // The daemon pushes metrics at 1 Hz and bursts device events. A transport
  // that cannot stream cleanly is unusable regardless of unary performance.
  stdout.writeln('');
  stdout.writeln('[3/3] server streaming (1000 messages x 512B)');
  try {
    const count = 1000;
    const payloadBytes = 512;
    var received = 0;
    var bytes = 0;
    var outOfOrder = 0;
    var expectedSeq = 0;
    final deliveryLatency = <int>[];

    final sw = Stopwatch()..start();
    final stream = client.ticks(TicksRequest()
      ..count = count
      ..payloadBytes = payloadBytes);

    await for (final tick in stream) {
      if (tick.seq.toInt() != expectedSeq) outOfOrder++;
      expectedSeq++;
      received++;
      bytes += tick.payload.length;

      final nowMicros = DateTime.now().microsecondsSinceEpoch;
      deliveryLatency.add(nowMicros - tick.unixMicros.toInt());
    }
    sw.stop();

    final seconds = sw.elapsedMicroseconds / 1e6;
    final msgPerSec = received / seconds;
    final mibPerSec = (bytes / seconds) / (1024 * 1024);

    stdout.writeln('  received:   $received/$count messages, '
        '${(bytes / 1024).toStringAsFixed(0)} KiB');
    stdout.writeln('  elapsed:    ${seconds.toStringAsFixed(3)} s');
    stdout.writeln('  throughput: ${msgPerSec.toStringAsFixed(0)} msg/s, '
        '${mibPerSec.toStringAsFixed(1)} MiB/s');
    stdout.writeln('  ordering:   ${outOfOrder == 0 ? "in order" : "$outOfOrder OUT OF ORDER"}');

    deliveryLatency.sort();
    stdout.writeln('  delivery p50: '
        '${_fmtMicros(deliveryLatency[deliveryLatency.length ~/ 2])}');

    if (received != count) {
      stdout.writeln('  FAIL: message loss');
      failures++;
    }
    if (outOfOrder != 0) {
      stdout.writeln('  FAIL: stream reordering');
      failures++;
    }
    if (received == count && outOfOrder == 0) {
      stdout.writeln('  OK:     streaming is lossless and ordered');
    }
  } catch (e) {
    stdout.writeln('  FAIL: $e');
    failures++;
  }

  stdout.writeln('');
  stdout.writeln(failures == 0
      ? 'SPIKE-A: PASS — Dart speaks gRPC over this transport.'
      : 'SPIKE-A: FAIL — $failures problem(s).');
  return failures == 0 ? 0 : 1;
}

/// The uid Dart is running as, for cross-checking what the server reported.
/// Returns null on platforms where `id -u` is not meaningful.
int? _currentUid() {
  if (Platform.isWindows) return null;
  try {
    final r = Process.runSync('id', ['-u']);
    return int.tryParse((r.stdout as String).trim());
  } catch (_) {
    return null;
  }
}

String _fmtMicros(int micros) => micros < 1000
    ? '$micros us'
    : '${(micros / 1000).toStringAsFixed(2)} ms';
