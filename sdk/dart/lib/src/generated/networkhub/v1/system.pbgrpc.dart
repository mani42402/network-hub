// This is a generated file - do not edit.
//
// Generated from networkhub/v1/system.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:async' as $async;
import 'dart:core' as $core;

import 'package:grpc/service_api.dart' as $grpc;
import 'package:protobuf/protobuf.dart' as $pb;

import 'system.pb.dart' as $0;

export 'system.pb.dart';

/// System-level queries: version negotiation, health, and the capability matrix
/// (AD-6). This is the first service any client calls — a client checks version
/// compatibility and reads capabilities before rendering anything.
@$pb.GrpcServiceName('networkhub.v1.SystemService')
class SystemServiceClient extends $grpc.Client {
  /// The hostname for this service.
  static const $core.String defaultHost = '';

  /// OAuth scopes needed for the client.
  static const $core.List<$core.String> oauthScopes = [
    '',
  ];

  SystemServiceClient(super.channel, {super.options, super.interceptors});

  /// Returns daemon and API version information, so a client can detect a
  /// version mismatch and show an "update the app / update the service" screen
  /// rather than behaving undefined (ADR-0013).
  $grpc.ResponseFuture<$0.GetVersionResponse> getVersion(
    $0.GetVersionRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getVersion, request, options: options);
  }

  /// Returns a point-in-time health summary of the daemon and its modules.
  $grpc.ResponseFuture<$0.GetHealthResponse> getHealth(
    $0.GetHealthRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getHealth, request, options: options);
  }

  /// Returns the full capability matrix for this machine (AD-6). The UI renders
  /// features from this; unsupported entries carry the reason they cannot work.
  $grpc.ResponseFuture<$0.GetCapabilitiesResponse> getCapabilities(
    $0.GetCapabilitiesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getCapabilities, request, options: options);
  }

  // method descriptors

  static final _$getVersion =
      $grpc.ClientMethod<$0.GetVersionRequest, $0.GetVersionResponse>(
          '/networkhub.v1.SystemService/GetVersion',
          ($0.GetVersionRequest value) => value.writeToBuffer(),
          $0.GetVersionResponse.fromBuffer);
  static final _$getHealth =
      $grpc.ClientMethod<$0.GetHealthRequest, $0.GetHealthResponse>(
          '/networkhub.v1.SystemService/GetHealth',
          ($0.GetHealthRequest value) => value.writeToBuffer(),
          $0.GetHealthResponse.fromBuffer);
  static final _$getCapabilities =
      $grpc.ClientMethod<$0.GetCapabilitiesRequest, $0.GetCapabilitiesResponse>(
          '/networkhub.v1.SystemService/GetCapabilities',
          ($0.GetCapabilitiesRequest value) => value.writeToBuffer(),
          $0.GetCapabilitiesResponse.fromBuffer);
}

@$pb.GrpcServiceName('networkhub.v1.SystemService')
abstract class SystemServiceBase extends $grpc.Service {
  $core.String get $name => 'networkhub.v1.SystemService';

  SystemServiceBase() {
    $addMethod($grpc.ServiceMethod<$0.GetVersionRequest, $0.GetVersionResponse>(
        'GetVersion',
        getVersion_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.GetVersionRequest.fromBuffer(value),
        ($0.GetVersionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetHealthRequest, $0.GetHealthResponse>(
        'GetHealth',
        getHealth_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.GetHealthRequest.fromBuffer(value),
        ($0.GetHealthResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetCapabilitiesRequest,
            $0.GetCapabilitiesResponse>(
        'GetCapabilities',
        getCapabilities_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.GetCapabilitiesRequest.fromBuffer(value),
        ($0.GetCapabilitiesResponse value) => value.writeToBuffer()));
  }

  $async.Future<$0.GetVersionResponse> getVersion_Pre($grpc.ServiceCall $call,
      $async.Future<$0.GetVersionRequest> $request) async {
    return getVersion($call, await $request);
  }

  $async.Future<$0.GetVersionResponse> getVersion(
      $grpc.ServiceCall call, $0.GetVersionRequest request);

  $async.Future<$0.GetHealthResponse> getHealth_Pre($grpc.ServiceCall $call,
      $async.Future<$0.GetHealthRequest> $request) async {
    return getHealth($call, await $request);
  }

  $async.Future<$0.GetHealthResponse> getHealth(
      $grpc.ServiceCall call, $0.GetHealthRequest request);

  $async.Future<$0.GetCapabilitiesResponse> getCapabilities_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.GetCapabilitiesRequest> $request) async {
    return getCapabilities($call, await $request);
  }

  $async.Future<$0.GetCapabilitiesResponse> getCapabilities(
      $grpc.ServiceCall call, $0.GetCapabilitiesRequest request);
}
