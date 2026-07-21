// This is a generated file - do not edit.
//
// Generated from networkhub/v1/system.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:protobuf/protobuf.dart' as $pb;

import 'common.pb.dart' as $1;

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

class GetVersionRequest extends $pb.GeneratedMessage {
  factory GetVersionRequest() => create();

  GetVersionRequest._();

  factory GetVersionRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetVersionRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetVersionRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetVersionRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetVersionRequest copyWith(void Function(GetVersionRequest) updates) =>
      super.copyWith((message) => updates(message as GetVersionRequest))
          as GetVersionRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetVersionRequest create() => GetVersionRequest._();
  @$core.override
  GetVersionRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetVersionRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetVersionRequest>(create);
  static GetVersionRequest? _defaultInstance;
}

class GetVersionResponse extends $pb.GeneratedMessage {
  factory GetVersionResponse({
    $core.String? daemonVersion,
    $core.String? apiVersion,
    $core.String? minClientVersion,
    $core.String? os,
  }) {
    final result = create();
    if (daemonVersion != null) result.daemonVersion = daemonVersion;
    if (apiVersion != null) result.apiVersion = apiVersion;
    if (minClientVersion != null) result.minClientVersion = minClientVersion;
    if (os != null) result.os = os;
    return result;
  }

  GetVersionResponse._();

  factory GetVersionResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetVersionResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetVersionResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'daemonVersion')
    ..aOS(2, _omitFieldNames ? '' : 'apiVersion')
    ..aOS(3, _omitFieldNames ? '' : 'minClientVersion')
    ..aOS(4, _omitFieldNames ? '' : 'os')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetVersionResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetVersionResponse copyWith(void Function(GetVersionResponse) updates) =>
      super.copyWith((message) => updates(message as GetVersionResponse))
          as GetVersionResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetVersionResponse create() => GetVersionResponse._();
  @$core.override
  GetVersionResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetVersionResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetVersionResponse>(create);
  static GetVersionResponse? _defaultInstance;

  /// The daemon's own version, e.g. "0.1.0".
  @$pb.TagNumber(1)
  $core.String get daemonVersion => $_getSZ(0);
  @$pb.TagNumber(1)
  set daemonVersion($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasDaemonVersion() => $_has(0);
  @$pb.TagNumber(1)
  void clearDaemonVersion() => $_clearField(1);

  /// The API contract version this daemon speaks, e.g. "v1".
  @$pb.TagNumber(2)
  $core.String get apiVersion => $_getSZ(1);
  @$pb.TagNumber(2)
  set apiVersion($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasApiVersion() => $_has(1);
  @$pb.TagNumber(2)
  void clearApiVersion() => $_clearField(2);

  /// The oldest client the daemon will serve. A client older than this must
  /// prompt the user to update (ADR-0013).
  @$pb.TagNumber(3)
  $core.String get minClientVersion => $_getSZ(2);
  @$pb.TagNumber(3)
  set minClientVersion($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasMinClientVersion() => $_has(2);
  @$pb.TagNumber(3)
  void clearMinClientVersion() => $_clearField(3);

  /// Operating system the daemon runs on: "linux", "windows", or "macos".
  @$pb.TagNumber(4)
  $core.String get os => $_getSZ(3);
  @$pb.TagNumber(4)
  set os($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasOs() => $_has(3);
  @$pb.TagNumber(4)
  void clearOs() => $_clearField(4);
}

class GetHealthRequest extends $pb.GeneratedMessage {
  factory GetHealthRequest() => create();

  GetHealthRequest._();

  factory GetHealthRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetHealthRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetHealthRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetHealthRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetHealthRequest copyWith(void Function(GetHealthRequest) updates) =>
      super.copyWith((message) => updates(message as GetHealthRequest))
          as GetHealthRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetHealthRequest create() => GetHealthRequest._();
  @$core.override
  GetHealthRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetHealthRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetHealthRequest>(create);
  static GetHealthRequest? _defaultInstance;
}

/// Overall daemon health, plus a per-module breakdown so a degraded module
/// (e.g. a failed hotspot reconciler) is visible without taking down the rest.
class GetHealthResponse extends $pb.GeneratedMessage {
  factory GetHealthResponse({
    $1.ServingStatus? overall,
    $core.Iterable<ModuleHealth>? modules,
  }) {
    final result = create();
    if (overall != null) result.overall = overall;
    if (modules != null) result.modules.addAll(modules);
    return result;
  }

  GetHealthResponse._();

  factory GetHealthResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetHealthResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetHealthResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aE<$1.ServingStatus>(1, _omitFieldNames ? '' : 'overall',
        enumValues: $1.ServingStatus.values)
    ..pPM<ModuleHealth>(2, _omitFieldNames ? '' : 'modules',
        subBuilder: ModuleHealth.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetHealthResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetHealthResponse copyWith(void Function(GetHealthResponse) updates) =>
      super.copyWith((message) => updates(message as GetHealthResponse))
          as GetHealthResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetHealthResponse create() => GetHealthResponse._();
  @$core.override
  GetHealthResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetHealthResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetHealthResponse>(create);
  static GetHealthResponse? _defaultInstance;

  /// ServingStatus is defined in common.proto (shared with the event stream).
  @$pb.TagNumber(1)
  $1.ServingStatus get overall => $_getN(0);
  @$pb.TagNumber(1)
  set overall($1.ServingStatus value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasOverall() => $_has(0);
  @$pb.TagNumber(1)
  void clearOverall() => $_clearField(1);

  @$pb.TagNumber(2)
  $pb.PbList<ModuleHealth> get modules => $_getList(1);
}

class ModuleHealth extends $pb.GeneratedMessage {
  factory ModuleHealth({
    $core.String? name,
    $1.ServingStatus? status,
    $core.String? detail,
  }) {
    final result = create();
    if (name != null) result.name = name;
    if (status != null) result.status = status;
    if (detail != null) result.detail = detail;
    return result;
  }

  ModuleHealth._();

  factory ModuleHealth.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ModuleHealth.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ModuleHealth',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'name')
    ..aE<$1.ServingStatus>(2, _omitFieldNames ? '' : 'status',
        enumValues: $1.ServingStatus.values)
    ..aOS(3, _omitFieldNames ? '' : 'detail')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ModuleHealth clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ModuleHealth copyWith(void Function(ModuleHealth) updates) =>
      super.copyWith((message) => updates(message as ModuleHealth))
          as ModuleHealth;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ModuleHealth create() => ModuleHealth._();
  @$core.override
  ModuleHealth createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ModuleHealth getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ModuleHealth>(create);
  static ModuleHealth? _defaultInstance;

  /// Module name, e.g. "monitor", "hotspot", "vpn".
  @$pb.TagNumber(1)
  $core.String get name => $_getSZ(0);
  @$pb.TagNumber(1)
  set name($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasName() => $_has(0);
  @$pb.TagNumber(1)
  void clearName() => $_clearField(1);

  @$pb.TagNumber(2)
  $1.ServingStatus get status => $_getN(1);
  @$pb.TagNumber(2)
  set status($1.ServingStatus value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasStatus() => $_has(1);
  @$pb.TagNumber(2)
  void clearStatus() => $_clearField(2);

  /// Human explanation when not fully serving; empty when healthy.
  @$pb.TagNumber(3)
  $core.String get detail => $_getSZ(2);
  @$pb.TagNumber(3)
  set detail($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasDetail() => $_has(2);
  @$pb.TagNumber(3)
  void clearDetail() => $_clearField(3);
}

class GetCapabilitiesRequest extends $pb.GeneratedMessage {
  factory GetCapabilitiesRequest() => create();

  GetCapabilitiesRequest._();

  factory GetCapabilitiesRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetCapabilitiesRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetCapabilitiesRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetCapabilitiesRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetCapabilitiesRequest copyWith(
          void Function(GetCapabilitiesRequest) updates) =>
      super.copyWith((message) => updates(message as GetCapabilitiesRequest))
          as GetCapabilitiesRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetCapabilitiesRequest create() => GetCapabilitiesRequest._();
  @$core.override
  GetCapabilitiesRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetCapabilitiesRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetCapabilitiesRequest>(create);
  static GetCapabilitiesRequest? _defaultInstance;
}

class GetCapabilitiesResponse extends $pb.GeneratedMessage {
  factory GetCapabilitiesResponse({
    $core.Iterable<$core.MapEntry<$core.String, $1.Capability>>? capabilities,
  }) {
    final result = create();
    if (capabilities != null) result.capabilities.addEntries(capabilities);
    return result;
  }

  GetCapabilitiesResponse._();

  factory GetCapabilitiesResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory GetCapabilitiesResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetCapabilitiesResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..m<$core.String, $1.Capability>(1, _omitFieldNames ? '' : 'capabilities',
        entryClassName: 'GetCapabilitiesResponse.CapabilitiesEntry',
        keyFieldType: $pb.PbFieldType.OS,
        valueFieldType: $pb.PbFieldType.OM,
        valueCreator: $1.Capability.create,
        valueDefaultOrMaker: $1.Capability.getDefault,
        packageName: const $pb.PackageName('networkhub.v1'))
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetCapabilitiesResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetCapabilitiesResponse copyWith(
          void Function(GetCapabilitiesResponse) updates) =>
      super.copyWith((message) => updates(message as GetCapabilitiesResponse))
          as GetCapabilitiesResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static GetCapabilitiesResponse create() => GetCapabilitiesResponse._();
  @$core.override
  GetCapabilitiesResponse createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static GetCapabilitiesResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetCapabilitiesResponse>(create);
  static GetCapabilitiesResponse? _defaultInstance;

  /// Keyed by the stable dotted capability name, e.g. "hotspot.wifi_ap"
  /// (ARCHITECTURE.md §3.4).
  @$pb.TagNumber(1)
  $pb.PbMap<$core.String, $1.Capability> get capabilities => $_getMap(0);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
