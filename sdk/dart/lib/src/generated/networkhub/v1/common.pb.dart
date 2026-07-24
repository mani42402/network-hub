// This is a generated file - do not edit.
//
// Generated from networkhub/v1/common.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:fixnum/fixnum.dart' as $fixnum;
import 'package:protobuf/protobuf.dart' as $pb;

import 'common.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'common.pbenum.dart';

/// One entry in the capability matrix (AD-6). Mirrors nh_types::Capability.
class Capability extends $pb.GeneratedMessage {
  factory Capability({
    CapabilityStatus? status,
    $core.String? reason,
    $core.Iterable<$core.MapEntry<$core.String, $core.String>>? details,
  }) {
    final result = create();
    if (status != null) result.status = status;
    if (reason != null) result.reason = reason;
    if (details != null) result.details.addEntries(details);
    return result;
  }

  Capability._();

  factory Capability.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory Capability.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Capability',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aE<CapabilityStatus>(1, _omitFieldNames ? '' : 'status',
        enumValues: CapabilityStatus.values)
    ..aOS(2, _omitFieldNames ? '' : 'reason')
    ..m<$core.String, $core.String>(3, _omitFieldNames ? '' : 'details',
        entryClassName: 'Capability.DetailsEntry',
        keyFieldType: $pb.PbFieldType.OS,
        valueFieldType: $pb.PbFieldType.OS,
        packageName: const $pb.PackageName('networkhub.v1'))
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Capability clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Capability copyWith(void Function(Capability) updates) =>
      super.copyWith((message) => updates(message as Capability)) as Capability;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static Capability create() => Capability._();
  @$core.override
  Capability createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static Capability getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<Capability>(create);
  static Capability? _defaultInstance;

  @$pb.TagNumber(1)
  CapabilityStatus get status => $_getN(0);
  @$pb.TagNumber(1)
  set status(CapabilityStatus value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasStatus() => $_has(0);
  @$pb.TagNumber(1)
  void clearStatus() => $_clearField(1);

  /// Human-readable explanation, shown in the UI. Always present.
  @$pb.TagNumber(2)
  $core.String get reason => $_getSZ(1);
  @$pb.TagNumber(2)
  set reason($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasReason() => $_has(1);
  @$pb.TagNumber(2)
  void clearReason() => $_clearField(2);

  /// Optional structured specifics (supported bands, client-count caps, the name
  /// of a conflicting service).
  @$pb.TagNumber(3)
  $pb.PbMap<$core.String, $core.String> get details => $_getMap(2);
}

/// The error contract that accompanies a failed RPC in its status details
/// (ARCHITECTURE.md §6.3). Mirrors nh_types::DomainError. The transport status
/// code carries the coarse ErrorCode; this message carries the specifics the UI
/// renders.
class ErrorInfo extends $pb.GeneratedMessage {
  factory ErrorInfo({
    $core.String? reason,
    $core.String? detail,
    $core.String? remediation,
  }) {
    final result = create();
    if (reason != null) result.reason = reason;
    if (detail != null) result.detail = detail;
    if (remediation != null) result.remediation = remediation;
    return result;
  }

  ErrorInfo._();

  factory ErrorInfo.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ErrorInfo.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ErrorInfo',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'reason')
    ..aOS(2, _omitFieldNames ? '' : 'detail')
    ..aOS(3, _omitFieldNames ? '' : 'remediation')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ErrorInfo clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ErrorInfo copyWith(void Function(ErrorInfo) updates) =>
      super.copyWith((message) => updates(message as ErrorInfo)) as ErrorInfo;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ErrorInfo create() => ErrorInfo._();
  @$core.override
  ErrorInfo createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ErrorInfo getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ErrorInfo>(create);
  static ErrorInfo? _defaultInstance;

  /// Stable machine-readable slug, e.g. "hotspot.subnet_conflict".
  @$pb.TagNumber(1)
  $core.String get reason => $_getSZ(0);
  @$pb.TagNumber(1)
  set reason($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasReason() => $_has(0);
  @$pb.TagNumber(1)
  void clearReason() => $_clearField(1);

  /// Human sentence describing what went wrong. Always present.
  @$pb.TagNumber(2)
  $core.String get detail => $_getSZ(1);
  @$pb.TagNumber(2)
  set detail($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasDetail() => $_has(1);
  @$pb.TagNumber(2)
  void clearDetail() => $_clearField(2);

  /// Optional human sentence telling the user how to fix it; the UI renders it.
  @$pb.TagNumber(3)
  $core.String get remediation => $_getSZ(2);
  @$pb.TagNumber(3)
  set remediation($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasRemediation() => $_has(2);
  @$pb.TagNumber(3)
  void clearRemediation() => $_clearField(3);
}

/// A quantity of bytes. Exact; formatting happens in clients.
class ByteCount extends $pb.GeneratedMessage {
  factory ByteCount({
    $fixnum.Int64? bytes,
  }) {
    final result = create();
    if (bytes != null) result.bytes = bytes;
    return result;
  }

  ByteCount._();

  factory ByteCount.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ByteCount.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ByteCount',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..a<$fixnum.Int64>(1, _omitFieldNames ? '' : 'bytes', $pb.PbFieldType.OU6,
        defaultOrMaker: $fixnum.Int64.ZERO)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ByteCount clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ByteCount copyWith(void Function(ByteCount) updates) =>
      super.copyWith((message) => updates(message as ByteCount)) as ByteCount;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ByteCount create() => ByteCount._();
  @$core.override
  ByteCount createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ByteCount getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ByteCount>(create);
  static ByteCount? _defaultInstance;

  @$pb.TagNumber(1)
  $fixnum.Int64 get bytes => $_getI64(0);
  @$pb.TagNumber(1)
  set bytes($fixnum.Int64 value) => $_setInt64(0, value);
  @$pb.TagNumber(1)
  $core.bool hasBytes() => $_has(0);
  @$pb.TagNumber(1)
  void clearBytes() => $_clearField(1);
}

/// A throughput in bits per second. Exact; clients format in decimal units.
class BitRate extends $pb.GeneratedMessage {
  factory BitRate({
    $fixnum.Int64? bitsPerSec,
  }) {
    final result = create();
    if (bitsPerSec != null) result.bitsPerSec = bitsPerSec;
    return result;
  }

  BitRate._();

  factory BitRate.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory BitRate.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'BitRate',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..a<$fixnum.Int64>(
        1, _omitFieldNames ? '' : 'bitsPerSec', $pb.PbFieldType.OU6,
        defaultOrMaker: $fixnum.Int64.ZERO)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  BitRate clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  BitRate copyWith(void Function(BitRate) updates) =>
      super.copyWith((message) => updates(message as BitRate)) as BitRate;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static BitRate create() => BitRate._();
  @$core.override
  BitRate createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static BitRate getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<BitRate>(create);
  static BitRate? _defaultInstance;

  @$pb.TagNumber(1)
  $fixnum.Int64 get bitsPerSec => $_getI64(0);
  @$pb.TagNumber(1)
  set bitsPerSec($fixnum.Int64 value) => $_setInt64(0, value);
  @$pb.TagNumber(1)
  $core.bool hasBitsPerSec() => $_has(0);
  @$pb.TagNumber(1)
  void clearBitsPerSec() => $_clearField(1);
}

class Common {
  static final requiredPermission = $pb.Extension<Permission>(
      _omitMessageNames ? '' : 'google.protobuf.MethodOptions',
      _omitFieldNames ? '' : 'requiredPermission',
      50001,
      $pb.PbFieldType.OE,
      defaultOrMaker: Permission.PERMISSION_UNSPECIFIED,
      valueOf: Permission.valueOf,
      enumValues: Permission.values);
  static void registerAllExtensions($pb.ExtensionRegistry registry) {
    registry.add(requiredPermission);
  }
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
