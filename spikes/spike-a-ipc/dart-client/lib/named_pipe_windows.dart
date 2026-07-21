// SPIKE-A option A2: a Windows named-pipe transport for grpc-dart, via dart:ffi.
//
// Why this file has to exist:
//   • dart:io has no named-pipe support, so a pipe cannot be opened as a Socket.
//   • Rust and tokio do not support AF_UNIX on Windows, so the server side is a
//     named pipe regardless of what Dart can do.
//
// grpc-dart does not require a real Socket. `ClientTransportConnection.viaStreams`
// accepts any Stream/StreamSink pair, so the job is to present a pipe handle as
// byte streams. That is what this does.
//
// Reads happen on a background isolate because ReadFile blocks, and blocking the
// main isolate would freeze the UI. Writes are issued inline: WriteFile to a
// pipe with a reading server returns promptly.
//
// Throwaway spike code. If A2 becomes the shipping path, the production version
// needs overlapped (async) I/O, cancellation, and a hardened error path.

import 'dart:async';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';
// ClientTransportConnector and ClientTransportConnectorChannel are both public
// exports of the top-level grpc library (verified against grpc 5.1.0), so no
// reaching into src/ is needed — which matters, because src/ imports are the
// kind of thing that breaks on a minor version bump.
import 'package:grpc/grpc.dart';
import 'package:http2/transport.dart' show ClientTransportConnection;

// --- Win32 bindings --------------------------------------------------------

const int _genericRead = 0x80000000;
const int _genericWrite = 0x40000000;
const int _openExisting = 3;
const int _invalidHandleValue = -1;
const int _errorPipeBusy = 231;

typedef _CreateFileWNative = IntPtr Function(
    Pointer<Utf16>, Uint32, Uint32, Pointer<Void>, Uint32, Uint32, IntPtr);
typedef _CreateFileWDart = int Function(
    Pointer<Utf16>, int, int, Pointer<Void>, int, int, int);

typedef _ReadFileNative = Int32 Function(
    IntPtr, Pointer<Uint8>, Uint32, Pointer<Uint32>, Pointer<Void>);
typedef _ReadFileDart = int Function(
    int, Pointer<Uint8>, int, Pointer<Uint32>, Pointer<Void>);

typedef _WriteFileNative = Int32 Function(
    IntPtr, Pointer<Uint8>, Uint32, Pointer<Uint32>, Pointer<Void>);
typedef _WriteFileDart = int Function(
    int, Pointer<Uint8>, int, Pointer<Uint32>, Pointer<Void>);

typedef _CloseHandleNative = Int32 Function(IntPtr);
typedef _CloseHandleDart = int Function(int);

typedef _WaitNamedPipeWNative = Int32 Function(Pointer<Utf16>, Uint32);
typedef _WaitNamedPipeWDart = int Function(Pointer<Utf16>, int);

typedef _GetLastErrorNative = Uint32 Function();
typedef _GetLastErrorDart = int Function();

class _Kernel32 {
  static final DynamicLibrary _lib = DynamicLibrary.open('kernel32.dll');

  static final createFileW =
      _lib.lookupFunction<_CreateFileWNative, _CreateFileWDart>('CreateFileW');
  static final readFile =
      _lib.lookupFunction<_ReadFileNative, _ReadFileDart>('ReadFile');
  static final writeFile =
      _lib.lookupFunction<_WriteFileNative, _WriteFileDart>('WriteFile');
  static final closeHandle =
      _lib.lookupFunction<_CloseHandleNative, _CloseHandleDart>('CloseHandle');
  static final waitNamedPipeW = _lib
      .lookupFunction<_WaitNamedPipeWNative, _WaitNamedPipeWDart>('WaitNamedPipeW');
  static final getLastError =
      _lib.lookupFunction<_GetLastErrorNative, _GetLastErrorDart>('GetLastError');
}

/// Opens a named pipe, waiting briefly if all server instances are busy.
int _openPipe(String name) {
  final namePtr = name.toNativeUtf16();
  try {
    for (var attempt = 0; attempt < 20; attempt++) {
      final handle = _Kernel32.createFileW(
        namePtr,
        _genericRead | _genericWrite,
        0, // no sharing
        nullptr,
        _openExisting,
        0,
        0,
      );
      if (handle != _invalidHandleValue) return handle;

      final err = _Kernel32.getLastError();
      if (err != _errorPipeBusy) {
        throw StateError('CreateFileW("$name") failed, GetLastError=$err');
      }
      // All instances busy — the server is between accepts. Wait and retry.
      _Kernel32.waitNamedPipeW(namePtr, 1000);
    }
    throw StateError('named pipe "$name" stayed busy after 20 attempts');
  } finally {
    calloc.free(namePtr);
  }
}

// --- Read isolate ----------------------------------------------------------

class _ReadIsolateArgs {
  final SendPort sendPort;
  final int handle;
  const _ReadIsolateArgs(this.sendPort, this.handle);
}

/// Blocking read loop. Sends Uint8List chunks until the pipe closes, then null.
void _readLoop(_ReadIsolateArgs args) {
  const bufSize = 64 * 1024;
  final buffer = calloc<Uint8>(bufSize);
  final bytesRead = calloc<Uint32>();
  try {
    while (true) {
      final ok = _Kernel32.readFile(args.handle, buffer, bufSize, bytesRead, nullptr);
      if (ok == 0 || bytesRead.value == 0) break;
      args.sendPort.send(
        Uint8List.fromList(buffer.asTypedList(bytesRead.value)),
      );
    }
  } finally {
    calloc.free(buffer);
    calloc.free(bytesRead);
    args.sendPort.send(null); // end of stream
  }
}

// --- Transport connector ---------------------------------------------------

/// Presents a Windows named pipe to grpc-dart as a bidirectional byte stream.
class NamedPipeTransportConnector implements ClientTransportConnector {
  final String pipeName;
  final _doneCompleter = Completer<void>();

  int? _handle;
  Isolate? _readIsolate;
  ReceivePort? _receivePort;

  NamedPipeTransportConnector(this.pipeName);

  @override
  String get authority => 'localhost';

  @override
  Future<void> get done => _doneCompleter.future;

  @override
  Future<ClientTransportConnection> connect() async {
    final handle = _openPipe(pipeName);
    _handle = handle;

    // Inbound: blocking reads on a background isolate.
    final receivePort = ReceivePort();
    _receivePort = receivePort;
    _readIsolate = await Isolate.spawn(
      _readLoop,
      _ReadIsolateArgs(receivePort.sendPort, handle),
    );

    final incoming = StreamController<List<int>>();
    receivePort.listen((message) {
      if (message == null) {
        incoming.close();
        if (!_doneCompleter.isCompleted) _doneCompleter.complete();
      } else if (message is Uint8List) {
        incoming.add(message);
      }
    });

    // Outbound: inline WriteFile.
    final outgoing = StreamController<List<int>>();
    outgoing.stream.listen(
      (chunk) => _write(handle, chunk),
      onDone: shutdown,
    );

    return ClientTransportConnection.viaStreams(incoming.stream, outgoing.sink);
  }

  void _write(int handle, List<int> chunk) {
    if (chunk.isEmpty) return;
    final buf = calloc<Uint8>(chunk.length);
    final written = calloc<Uint32>();
    try {
      buf.asTypedList(chunk.length).setAll(0, chunk);
      var offset = 0;
      while (offset < chunk.length) {
        final ok = _Kernel32.writeFile(
            handle, buf + offset, chunk.length - offset, written, nullptr);
        if (ok == 0) {
          throw StateError('WriteFile failed, GetLastError=${_Kernel32.getLastError()}');
        }
        offset += written.value;
      }
    } finally {
      calloc.free(buf);
      calloc.free(written);
    }
  }

  @override
  void shutdown() {
    _readIsolate?.kill(priority: Isolate.immediate);
    _receivePort?.close();
    final handle = _handle;
    if (handle != null) {
      _Kernel32.closeHandle(handle);
      _handle = null;
    }
    if (!_doneCompleter.isCompleted) _doneCompleter.complete();
  }
}

/// A grpc-dart channel backed by a Windows named pipe.
///
/// Note the returned type. grpc-dart exports *two* classes named `ClientChannel`:
/// a concrete socket-based one from `grpc.dart`, and an abstract interface from
/// `service_api.dart` that generated clients accept. This connector is a sibling
/// of the concrete socket class, not a subtype, so it cannot be typed as the
/// `grpc.dart` `ClientChannel`. `ClientTransportConnectorChannel` is the exported
/// concrete type and still satisfies the generated client's abstract interface.
ClientTransportConnectorChannel namedPipeChannel(String pipeName) {
  return ClientTransportConnectorChannel(
    NamedPipeTransportConnector(pipeName),
    options: const ChannelOptions(credentials: ChannelCredentials.insecure()),
  );
}
