// This is a generated file - do not edit.
//
// Generated from networkhub/v1/monitor.proto.

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

import 'monitor.pb.dart' as $0;

export 'monitor.pb.dart';

/// Live and historical monitoring. For M0 this is the network interface list and
/// a live throughput stream — the first real data the walking-skeleton UI renders
/// (WP-M0.10, WP-M0.12). Historical queries and per-device metrics arrive in M2.
@$pb.GrpcServiceName('networkhub.v1.MonitorService')
class MonitorServiceClient extends $grpc.Client {
  /// The hostname for this service.
  static const $core.String defaultHost = '';

  /// OAuth scopes needed for the client.
  static const $core.List<$core.String> oauthScopes = [
    '',
  ];

  MonitorServiceClient(super.channel, {super.options, super.interceptors});

  /// Lists the machine's network interfaces with their current addresses and
  /// link state. A point-in-time query; changes arrive via the event stream.
  $grpc.ResponseFuture<$0.ListInterfacesResponse> listInterfaces(
    $0.ListInterfacesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listInterfaces, request, options: options);
  }

  /// Streams live throughput samples. Follows the snapshot-first contract
  /// (events.proto): the first message carries current per-interface rates, then
  /// updates flow at the sampling interval.
  $grpc.ResponseStream<$0.ThroughputSample> streamThroughput(
    $0.StreamThroughputRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createStreamingCall(
        _$streamThroughput, $async.Stream.fromIterable([request]),
        options: options);
  }

  // method descriptors

  static final _$listInterfaces =
      $grpc.ClientMethod<$0.ListInterfacesRequest, $0.ListInterfacesResponse>(
          '/networkhub.v1.MonitorService/ListInterfaces',
          ($0.ListInterfacesRequest value) => value.writeToBuffer(),
          $0.ListInterfacesResponse.fromBuffer);
  static final _$streamThroughput =
      $grpc.ClientMethod<$0.StreamThroughputRequest, $0.ThroughputSample>(
          '/networkhub.v1.MonitorService/StreamThroughput',
          ($0.StreamThroughputRequest value) => value.writeToBuffer(),
          $0.ThroughputSample.fromBuffer);
}

@$pb.GrpcServiceName('networkhub.v1.MonitorService')
abstract class MonitorServiceBase extends $grpc.Service {
  $core.String get $name => 'networkhub.v1.MonitorService';

  MonitorServiceBase() {
    $addMethod($grpc.ServiceMethod<$0.ListInterfacesRequest,
            $0.ListInterfacesResponse>(
        'ListInterfaces',
        listInterfaces_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.ListInterfacesRequest.fromBuffer(value),
        ($0.ListInterfacesResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.StreamThroughputRequest, $0.ThroughputSample>(
            'StreamThroughput',
            streamThroughput_Pre,
            false,
            true,
            ($core.List<$core.int> value) =>
                $0.StreamThroughputRequest.fromBuffer(value),
            ($0.ThroughputSample value) => value.writeToBuffer()));
  }

  $async.Future<$0.ListInterfacesResponse> listInterfaces_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListInterfacesRequest> $request) async {
    return listInterfaces($call, await $request);
  }

  $async.Future<$0.ListInterfacesResponse> listInterfaces(
      $grpc.ServiceCall call, $0.ListInterfacesRequest request);

  $async.Stream<$0.ThroughputSample> streamThroughput_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.StreamThroughputRequest> $request) async* {
    yield* streamThroughput($call, await $request);
  }

  $async.Stream<$0.ThroughputSample> streamThroughput(
      $grpc.ServiceCall call, $0.StreamThroughputRequest request);
}
