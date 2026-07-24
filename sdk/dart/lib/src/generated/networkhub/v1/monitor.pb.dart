// This is a generated file - do not edit.
//
// Generated from networkhub/v1/monitor.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:fixnum/fixnum.dart' as $fixnum;
import 'package:protobuf/protobuf.dart' as $pb;

import 'common.pb.dart' as $1;
import 'monitor.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'monitor.pbenum.dart';

class ListInterfacesRequest extends $pb.GeneratedMessage {
  factory ListInterfacesRequest() => create();

  ListInterfacesRequest._();

  factory ListInterfacesRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ListInterfacesRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListInterfacesRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListInterfacesRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListInterfacesRequest copyWith(
          void Function(ListInterfacesRequest) updates) =>
      super.copyWith((message) => updates(message as ListInterfacesRequest))
          as ListInterfacesRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ListInterfacesRequest create() => ListInterfacesRequest._();
  @$core.override
  ListInterfacesRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ListInterfacesRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListInterfacesRequest>(create);
  static ListInterfacesRequest? _defaultInstance;
}

class ListInterfacesResponse extends $pb.GeneratedMessage {
  factory ListInterfacesResponse({
    $core.Iterable<Interface>? interfaces,
  }) {
    final result = create();
    if (interfaces != null) result.interfaces.addAll(interfaces);
    return result;
  }

  ListInterfacesResponse._();

  factory ListInterfacesResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ListInterfacesResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListInterfacesResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..pPM<Interface>(1, _omitFieldNames ? '' : 'interfaces',
        subBuilder: Interface.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListInterfacesResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListInterfacesResponse copyWith(
          void Function(ListInterfacesResponse) updates) =>
      super.copyWith((message) => updates(message as ListInterfacesResponse))
          as ListInterfacesResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ListInterfacesResponse create() => ListInterfacesResponse._();
  @$core.override
  ListInterfacesResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ListInterfacesResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListInterfacesResponse>(create);
  static ListInterfacesResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<Interface> get interfaces => $_getList(0);
}

/// A network interface. Identity is the stable `id`, not the OS name, because
/// names change across re-plug and predictable-naming schemes (F-13); the name
/// is shown to the user but not relied on.
class Interface extends $pb.GeneratedMessage {
  factory Interface({
    $core.String? id,
    $core.String? name,
    InterfaceKind? kind,
    $core.bool? isUp,
    $core.String? mac,
    $core.Iterable<$core.String>? addresses,
  }) {
    final result = create();
    if (id != null) result.id = id;
    if (name != null) result.name = name;
    if (kind != null) result.kind = kind;
    if (isUp != null) result.isUp = isUp;
    if (mac != null) result.mac = mac;
    if (addresses != null) result.addresses.addAll(addresses);
    return result;
  }

  Interface._();

  factory Interface.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory Interface.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Interface',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..aOS(2, _omitFieldNames ? '' : 'name')
    ..aE<InterfaceKind>(3, _omitFieldNames ? '' : 'kind',
        enumValues: InterfaceKind.values)
    ..aOB(4, _omitFieldNames ? '' : 'isUp')
    ..aOS(5, _omitFieldNames ? '' : 'mac')
    ..pPS(6, _omitFieldNames ? '' : 'addresses')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Interface clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Interface copyWith(void Function(Interface) updates) =>
      super.copyWith((message) => updates(message as Interface)) as Interface;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static Interface create() => Interface._();
  @$core.override
  Interface createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static Interface getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<Interface>(create);
  static Interface? _defaultInstance;

  /// Stable identity resolved by the daemon (F-13), safe to key on.
  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);

  /// Human-facing OS name, e.g. "en0", "eth0", "Wi-Fi".
  @$pb.TagNumber(2)
  $core.String get name => $_getSZ(1);
  @$pb.TagNumber(2)
  set name($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasName() => $_has(1);
  @$pb.TagNumber(2)
  void clearName() => $_clearField(2);

  @$pb.TagNumber(3)
  InterfaceKind get kind => $_getN(2);
  @$pb.TagNumber(3)
  set kind(InterfaceKind value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasKind() => $_has(2);
  @$pb.TagNumber(3)
  void clearKind() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.bool get isUp => $_getBF(3);
  @$pb.TagNumber(4)
  set isUp($core.bool value) => $_setBool(3, value);
  @$pb.TagNumber(4)
  $core.bool hasIsUp() => $_has(3);
  @$pb.TagNumber(4)
  void clearIsUp() => $_clearField(4);

  /// MAC address in canonical lower-case colon form, if the interface has one.
  @$pb.TagNumber(5)
  $core.String get mac => $_getSZ(4);
  @$pb.TagNumber(5)
  set mac($core.String value) => $_setString(4, value);
  @$pb.TagNumber(5)
  $core.bool hasMac() => $_has(4);
  @$pb.TagNumber(5)
  void clearMac() => $_clearField(5);

  /// Assigned IP addresses in CIDR form, e.g. "192.168.1.5/24", "fe80::1/64".
  @$pb.TagNumber(6)
  $pb.PbList<$core.String> get addresses => $_getList(5);
}

class StreamThroughputRequest extends $pb.GeneratedMessage {
  factory StreamThroughputRequest({
    $core.Iterable<$core.String>? interfaceIds,
  }) {
    final result = create();
    if (interfaceIds != null) result.interfaceIds.addAll(interfaceIds);
    return result;
  }

  StreamThroughputRequest._();

  factory StreamThroughputRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory StreamThroughputRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StreamThroughputRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..pPS(1, _omitFieldNames ? '' : 'interfaceIds')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StreamThroughputRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StreamThroughputRequest copyWith(
          void Function(StreamThroughputRequest) updates) =>
      super.copyWith((message) => updates(message as StreamThroughputRequest))
          as StreamThroughputRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static StreamThroughputRequest create() => StreamThroughputRequest._();
  @$core.override
  StreamThroughputRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static StreamThroughputRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StreamThroughputRequest>(create);
  static StreamThroughputRequest? _defaultInstance;

  /// Interface ids to report. Empty means all non-loopback interfaces.
  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get interfaceIds => $_getList(0);
}

/// One throughput sample across the requested interfaces. Sent at the sampling
/// interval; the first sample after subscription is the snapshot.
class ThroughputSample extends $pb.GeneratedMessage {
  factory ThroughputSample({
    $fixnum.Int64? unixMillis,
    $core.Iterable<InterfaceThroughput>? interfaces,
  }) {
    final result = create();
    if (unixMillis != null) result.unixMillis = unixMillis;
    if (interfaces != null) result.interfaces.addAll(interfaces);
    return result;
  }

  ThroughputSample._();

  factory ThroughputSample.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ThroughputSample.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ThroughputSample',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aInt64(1, _omitFieldNames ? '' : 'unixMillis')
    ..pPM<InterfaceThroughput>(2, _omitFieldNames ? '' : 'interfaces',
        subBuilder: InterfaceThroughput.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ThroughputSample clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ThroughputSample copyWith(void Function(ThroughputSample) updates) =>
      super.copyWith((message) => updates(message as ThroughputSample))
          as ThroughputSample;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ThroughputSample create() => ThroughputSample._();
  @$core.override
  ThroughputSample createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ThroughputSample getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ThroughputSample>(create);
  static ThroughputSample? _defaultInstance;

  /// Milliseconds since the Unix epoch, from a monotonic-corrected clock so a
  /// wall-clock jump does not produce absurd rates (E-26, F-11).
  @$pb.TagNumber(1)
  $fixnum.Int64 get unixMillis => $_getI64(0);
  @$pb.TagNumber(1)
  set unixMillis($fixnum.Int64 value) => $_setInt64(0, value);
  @$pb.TagNumber(1)
  $core.bool hasUnixMillis() => $_has(0);
  @$pb.TagNumber(1)
  void clearUnixMillis() => $_clearField(1);

  @$pb.TagNumber(2)
  $pb.PbList<InterfaceThroughput> get interfaces => $_getList(1);
}

class InterfaceThroughput extends $pb.GeneratedMessage {
  factory InterfaceThroughput({
    $core.String? interfaceId,
    $1.BitRate? rx,
    $1.BitRate? tx,
    $1.ByteCount? rxTotal,
    $1.ByteCount? txTotal,
  }) {
    final result = create();
    if (interfaceId != null) result.interfaceId = interfaceId;
    if (rx != null) result.rx = rx;
    if (tx != null) result.tx = tx;
    if (rxTotal != null) result.rxTotal = rxTotal;
    if (txTotal != null) result.txTotal = txTotal;
    return result;
  }

  InterfaceThroughput._();

  factory InterfaceThroughput.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory InterfaceThroughput.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'InterfaceThroughput',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'interfaceId')
    ..aOM<$1.BitRate>(2, _omitFieldNames ? '' : 'rx',
        subBuilder: $1.BitRate.create)
    ..aOM<$1.BitRate>(3, _omitFieldNames ? '' : 'tx',
        subBuilder: $1.BitRate.create)
    ..aOM<$1.ByteCount>(4, _omitFieldNames ? '' : 'rxTotal',
        subBuilder: $1.ByteCount.create)
    ..aOM<$1.ByteCount>(5, _omitFieldNames ? '' : 'txTotal',
        subBuilder: $1.ByteCount.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  InterfaceThroughput clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  InterfaceThroughput copyWith(void Function(InterfaceThroughput) updates) =>
      super.copyWith((message) => updates(message as InterfaceThroughput))
          as InterfaceThroughput;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static InterfaceThroughput create() => InterfaceThroughput._();
  @$core.override
  InterfaceThroughput createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static InterfaceThroughput getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<InterfaceThroughput>(create);
  static InterfaceThroughput? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get interfaceId => $_getSZ(0);
  @$pb.TagNumber(1)
  set interfaceId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasInterfaceId() => $_has(0);
  @$pb.TagNumber(1)
  void clearInterfaceId() => $_clearField(1);

  /// Instantaneous rates, derived from counter deltas with wrap detection so a
  /// rate is never negative (E-25).
  @$pb.TagNumber(2)
  $1.BitRate get rx => $_getN(1);
  @$pb.TagNumber(2)
  set rx($1.BitRate value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasRx() => $_has(1);
  @$pb.TagNumber(2)
  void clearRx() => $_clearField(2);
  @$pb.TagNumber(2)
  $1.BitRate ensureRx() => $_ensure(1);

  @$pb.TagNumber(3)
  $1.BitRate get tx => $_getN(2);
  @$pb.TagNumber(3)
  set tx($1.BitRate value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasTx() => $_has(2);
  @$pb.TagNumber(3)
  void clearTx() => $_clearField(3);
  @$pb.TagNumber(3)
  $1.BitRate ensureTx() => $_ensure(2);

  /// Cumulative counters since the interface came up, for totals.
  @$pb.TagNumber(4)
  $1.ByteCount get rxTotal => $_getN(3);
  @$pb.TagNumber(4)
  set rxTotal($1.ByteCount value) => $_setField(4, value);
  @$pb.TagNumber(4)
  $core.bool hasRxTotal() => $_has(3);
  @$pb.TagNumber(4)
  void clearRxTotal() => $_clearField(4);
  @$pb.TagNumber(4)
  $1.ByteCount ensureRxTotal() => $_ensure(3);

  @$pb.TagNumber(5)
  $1.ByteCount get txTotal => $_getN(4);
  @$pb.TagNumber(5)
  set txTotal($1.ByteCount value) => $_setField(5, value);
  @$pb.TagNumber(5)
  $core.bool hasTxTotal() => $_has(4);
  @$pb.TagNumber(5)
  void clearTxTotal() => $_clearField(5);
  @$pb.TagNumber(5)
  $1.ByteCount ensureTxTotal() => $_ensure(4);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
