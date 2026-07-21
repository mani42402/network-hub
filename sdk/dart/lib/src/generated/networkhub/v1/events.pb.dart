// This is a generated file - do not edit.
//
// Generated from networkhub/v1/events.proto.

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
import 'events.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'events.pbenum.dart';

class SubscribeRequest extends $pb.GeneratedMessage {
  factory SubscribeRequest({
    $core.Iterable<EventTopic>? topics,
  }) {
    final result = create();
    if (topics != null) result.topics.addAll(topics);
    return result;
  }

  SubscribeRequest._();

  factory SubscribeRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory SubscribeRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SubscribeRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..pc<EventTopic>(1, _omitFieldNames ? '' : 'topics', $pb.PbFieldType.KE,
        valueOf: EventTopic.valueOf,
        enumValues: EventTopic.values,
        defaultEnumValue: EventTopic.EVENT_TOPIC_UNSPECIFIED)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SubscribeRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SubscribeRequest copyWith(void Function(SubscribeRequest) updates) =>
      super.copyWith((message) => updates(message as SubscribeRequest))
          as SubscribeRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static SubscribeRequest create() => SubscribeRequest._();
  @$core.override
  SubscribeRequest createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static SubscribeRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SubscribeRequest>(create);
  static SubscribeRequest? _defaultInstance;

  /// Topics to receive. Empty means all topics the caller is permitted to see.
  /// A client is still delivered a snapshot scoped to these topics first.
  @$pb.TagNumber(1)
  $pb.PbList<EventTopic> get topics => $_getList(0);
}

enum StreamMessage_Payload { snapshot, delta, notSet }

/// The envelope every streamed message uses. Exactly one of snapshot/delta is
/// set, and `sequence` orders them.
class StreamMessage extends $pb.GeneratedMessage {
  factory StreamMessage({
    $fixnum.Int64? sequence,
    Snapshot? snapshot,
    Delta? delta,
  }) {
    final result = create();
    if (sequence != null) result.sequence = sequence;
    if (snapshot != null) result.snapshot = snapshot;
    if (delta != null) result.delta = delta;
    return result;
  }

  StreamMessage._();

  factory StreamMessage.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory StreamMessage.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static const $core.Map<$core.int, StreamMessage_Payload>
      _StreamMessage_PayloadByTag = {
    2: StreamMessage_Payload.snapshot,
    3: StreamMessage_Payload.delta,
    0: StreamMessage_Payload.notSet
  };
  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StreamMessage',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..oo(0, [2, 3])
    ..a<$fixnum.Int64>(
        1, _omitFieldNames ? '' : 'sequence', $pb.PbFieldType.OU6,
        defaultOrMaker: $fixnum.Int64.ZERO)
    ..aOM<Snapshot>(2, _omitFieldNames ? '' : 'snapshot',
        subBuilder: Snapshot.create)
    ..aOM<Delta>(3, _omitFieldNames ? '' : 'delta', subBuilder: Delta.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StreamMessage clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StreamMessage copyWith(void Function(StreamMessage) updates) =>
      super.copyWith((message) => updates(message as StreamMessage))
          as StreamMessage;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static StreamMessage create() => StreamMessage._();
  @$core.override
  StreamMessage createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static StreamMessage getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StreamMessage>(create);
  static StreamMessage? _defaultInstance;

  @$pb.TagNumber(2)
  @$pb.TagNumber(3)
  StreamMessage_Payload whichPayload() =>
      _StreamMessage_PayloadByTag[$_whichOneof(0)]!;
  @$pb.TagNumber(2)
  @$pb.TagNumber(3)
  void clearPayload() => $_clearField($_whichOneof(0));

  /// Monotonically increasing within a stream. The snapshot carries the sequence
  /// current at subscription time; each delta increments it. A gap means the
  /// client missed a delta and, if it cares, should re-subscribe for a fresh
  /// snapshot.
  @$pb.TagNumber(1)
  $fixnum.Int64 get sequence => $_getI64(0);
  @$pb.TagNumber(1)
  set sequence($fixnum.Int64 value) => $_setInt64(0, value);
  @$pb.TagNumber(1)
  $core.bool hasSequence() => $_has(0);
  @$pb.TagNumber(1)
  void clearSequence() => $_clearField(1);

  /// Always the first message on a stream: the complete current state for the
  /// subscribed topics.
  @$pb.TagNumber(2)
  Snapshot get snapshot => $_getN(1);
  @$pb.TagNumber(2)
  set snapshot(Snapshot value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasSnapshot() => $_has(1);
  @$pb.TagNumber(2)
  void clearSnapshot() => $_clearField(2);
  @$pb.TagNumber(2)
  Snapshot ensureSnapshot() => $_ensure(1);

  /// An incremental change applied on top of the snapshot.
  @$pb.TagNumber(3)
  Delta get delta => $_getN(2);
  @$pb.TagNumber(3)
  set delta(Delta value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasDelta() => $_has(2);
  @$pb.TagNumber(3)
  void clearDelta() => $_clearField(3);
  @$pb.TagNumber(3)
  Delta ensureDelta() => $_ensure(2);
}

/// The complete current state at subscription time. For M0 this carries only the
/// capability matrix and module health; feature milestones extend it (devices,
/// VPN, hotspot) as those modules land.
class Snapshot extends $pb.GeneratedMessage {
  factory Snapshot({
    $core.Iterable<$core.MapEntry<$core.String, $1.Capability>>? capabilities,
    $core.Iterable<ModuleHealthEvent>? moduleHealth,
  }) {
    final result = create();
    if (capabilities != null) result.capabilities.addEntries(capabilities);
    if (moduleHealth != null) result.moduleHealth.addAll(moduleHealth);
    return result;
  }

  Snapshot._();

  factory Snapshot.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory Snapshot.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Snapshot',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..m<$core.String, $1.Capability>(1, _omitFieldNames ? '' : 'capabilities',
        entryClassName: 'Snapshot.CapabilitiesEntry',
        keyFieldType: $pb.PbFieldType.OS,
        valueFieldType: $pb.PbFieldType.OM,
        valueCreator: $1.Capability.create,
        valueDefaultOrMaker: $1.Capability.getDefault,
        packageName: const $pb.PackageName('networkhub.v1'))
    ..pPM<ModuleHealthEvent>(2, _omitFieldNames ? '' : 'moduleHealth',
        subBuilder: ModuleHealthEvent.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Snapshot clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Snapshot copyWith(void Function(Snapshot) updates) =>
      super.copyWith((message) => updates(message as Snapshot)) as Snapshot;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static Snapshot create() => Snapshot._();
  @$core.override
  Snapshot createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static Snapshot getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<Snapshot>(create);
  static Snapshot? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbMap<$core.String, $1.Capability> get capabilities => $_getMap(0);

  @$pb.TagNumber(2)
  $pb.PbList<ModuleHealthEvent> get moduleHealth => $_getList(1);
}

enum Delta_Event {
  capabilityChanged,
  moduleHealthChanged,
  conflictDetected,
  notSet
}

/// A single incremental change. Exactly one field is set. New event kinds are
/// added here additively as features land.
class Delta extends $pb.GeneratedMessage {
  factory Delta({
    CapabilityChanged? capabilityChanged,
    ModuleHealthEvent? moduleHealthChanged,
    ConflictDetected? conflictDetected,
  }) {
    final result = create();
    if (capabilityChanged != null) result.capabilityChanged = capabilityChanged;
    if (moduleHealthChanged != null)
      result.moduleHealthChanged = moduleHealthChanged;
    if (conflictDetected != null) result.conflictDetected = conflictDetected;
    return result;
  }

  Delta._();

  factory Delta.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory Delta.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static const $core.Map<$core.int, Delta_Event> _Delta_EventByTag = {
    1: Delta_Event.capabilityChanged,
    2: Delta_Event.moduleHealthChanged,
    3: Delta_Event.conflictDetected,
    0: Delta_Event.notSet
  };
  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Delta',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..oo(0, [1, 2, 3])
    ..aOM<CapabilityChanged>(1, _omitFieldNames ? '' : 'capabilityChanged',
        subBuilder: CapabilityChanged.create)
    ..aOM<ModuleHealthEvent>(2, _omitFieldNames ? '' : 'moduleHealthChanged',
        subBuilder: ModuleHealthEvent.create)
    ..aOM<ConflictDetected>(3, _omitFieldNames ? '' : 'conflictDetected',
        subBuilder: ConflictDetected.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Delta clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Delta copyWith(void Function(Delta) updates) =>
      super.copyWith((message) => updates(message as Delta)) as Delta;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static Delta create() => Delta._();
  @$core.override
  Delta createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static Delta getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<Delta>(create);
  static Delta? _defaultInstance;

  @$pb.TagNumber(1)
  @$pb.TagNumber(2)
  @$pb.TagNumber(3)
  Delta_Event whichEvent() => _Delta_EventByTag[$_whichOneof(0)]!;
  @$pb.TagNumber(1)
  @$pb.TagNumber(2)
  @$pb.TagNumber(3)
  void clearEvent() => $_clearField($_whichOneof(0));

  @$pb.TagNumber(1)
  CapabilityChanged get capabilityChanged => $_getN(0);
  @$pb.TagNumber(1)
  set capabilityChanged(CapabilityChanged value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasCapabilityChanged() => $_has(0);
  @$pb.TagNumber(1)
  void clearCapabilityChanged() => $_clearField(1);
  @$pb.TagNumber(1)
  CapabilityChanged ensureCapabilityChanged() => $_ensure(0);

  @$pb.TagNumber(2)
  ModuleHealthEvent get moduleHealthChanged => $_getN(1);
  @$pb.TagNumber(2)
  set moduleHealthChanged(ModuleHealthEvent value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasModuleHealthChanged() => $_has(1);
  @$pb.TagNumber(2)
  void clearModuleHealthChanged() => $_clearField(2);
  @$pb.TagNumber(2)
  ModuleHealthEvent ensureModuleHealthChanged() => $_ensure(1);

  @$pb.TagNumber(3)
  ConflictDetected get conflictDetected => $_getN(2);
  @$pb.TagNumber(3)
  set conflictDetected(ConflictDetected value) => $_setField(3, value);
  @$pb.TagNumber(3)
  $core.bool hasConflictDetected() => $_has(2);
  @$pb.TagNumber(3)
  void clearConflictDetected() => $_clearField(3);
  @$pb.TagNumber(3)
  ConflictDetected ensureConflictDetected() => $_ensure(2);
}

/// A capability's status changed (e.g. a USB Wi-Fi adapter was plugged in).
class CapabilityChanged extends $pb.GeneratedMessage {
  factory CapabilityChanged({
    $core.String? key,
    $1.Capability? capability,
  }) {
    final result = create();
    if (key != null) result.key = key;
    if (capability != null) result.capability = capability;
    return result;
  }

  CapabilityChanged._();

  factory CapabilityChanged.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory CapabilityChanged.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'CapabilityChanged',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'key')
    ..aOM<$1.Capability>(2, _omitFieldNames ? '' : 'capability',
        subBuilder: $1.Capability.create)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  CapabilityChanged clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  CapabilityChanged copyWith(void Function(CapabilityChanged) updates) =>
      super.copyWith((message) => updates(message as CapabilityChanged))
          as CapabilityChanged;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static CapabilityChanged create() => CapabilityChanged._();
  @$core.override
  CapabilityChanged createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static CapabilityChanged getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<CapabilityChanged>(create);
  static CapabilityChanged? _defaultInstance;

  /// The dotted capability key, e.g. "hotspot.wifi_ap".
  @$pb.TagNumber(1)
  $core.String get key => $_getSZ(0);
  @$pb.TagNumber(1)
  set key($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasKey() => $_has(0);
  @$pb.TagNumber(1)
  void clearKey() => $_clearField(1);

  @$pb.TagNumber(2)
  $1.Capability get capability => $_getN(1);
  @$pb.TagNumber(2)
  set capability($1.Capability value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasCapability() => $_has(1);
  @$pb.TagNumber(2)
  void clearCapability() => $_clearField(2);
  @$pb.TagNumber(2)
  $1.Capability ensureCapability() => $_ensure(1);
}

/// A module's health changed.
class ModuleHealthEvent extends $pb.GeneratedMessage {
  factory ModuleHealthEvent({
    $core.String? module,
    $1.ServingStatus? status,
    $core.String? detail,
  }) {
    final result = create();
    if (module != null) result.module = module;
    if (status != null) result.status = status;
    if (detail != null) result.detail = detail;
    return result;
  }

  ModuleHealthEvent._();

  factory ModuleHealthEvent.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ModuleHealthEvent.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ModuleHealthEvent',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'module')
    ..aE<$1.ServingStatus>(2, _omitFieldNames ? '' : 'status',
        enumValues: $1.ServingStatus.values)
    ..aOS(3, _omitFieldNames ? '' : 'detail')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ModuleHealthEvent clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ModuleHealthEvent copyWith(void Function(ModuleHealthEvent) updates) =>
      super.copyWith((message) => updates(message as ModuleHealthEvent))
          as ModuleHealthEvent;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ModuleHealthEvent create() => ModuleHealthEvent._();
  @$core.override
  ModuleHealthEvent createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ModuleHealthEvent getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ModuleHealthEvent>(create);
  static ModuleHealthEvent? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get module => $_getSZ(0);
  @$pb.TagNumber(1)
  set module($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasModule() => $_has(0);
  @$pb.TagNumber(1)
  void clearModule() => $_clearField(1);

  @$pb.TagNumber(2)
  $1.ServingStatus get status => $_getN(1);
  @$pb.TagNumber(2)
  set status($1.ServingStatus value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasStatus() => $_has(1);
  @$pb.TagNumber(2)
  void clearStatus() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get detail => $_getSZ(2);
  @$pb.TagNumber(3)
  set detail($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasDetail() => $_has(2);
  @$pb.TagNumber(3)
  void clearDetail() => $_clearField(3);
}

/// Another network agent (firewalld, an antivirus product, a policy script) was
/// detected repeatedly reverting NetworkHub state; the reconciler stopped
/// fighting and marked the module conflicted (§5.3).
class ConflictDetected extends $pb.GeneratedMessage {
  factory ConflictDetected({
    $core.String? module,
    $core.String? detail,
  }) {
    final result = create();
    if (module != null) result.module = module;
    if (detail != null) result.detail = detail;
    return result;
  }

  ConflictDetected._();

  factory ConflictDetected.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromBuffer(data, registry);
  factory ConflictDetected.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      create()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ConflictDetected',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'networkhub.v1'),
      createEmptyInstance: create)
    ..aOS(1, _omitFieldNames ? '' : 'module')
    ..aOS(2, _omitFieldNames ? '' : 'detail')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ConflictDetected clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ConflictDetected copyWith(void Function(ConflictDetected) updates) =>
      super.copyWith((message) => updates(message as ConflictDetected))
          as ConflictDetected;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  static ConflictDetected create() => ConflictDetected._();
  @$core.override
  ConflictDetected createEmptyInstance() => create();
  @$core.pragma('dart2js:noInline')
  static ConflictDetected getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ConflictDetected>(create);
  static ConflictDetected? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get module => $_getSZ(0);
  @$pb.TagNumber(1)
  set module($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasModule() => $_has(0);
  @$pb.TagNumber(1)
  void clearModule() => $_clearField(1);

  /// A human description of the apparent conflict and the suspected cause.
  @$pb.TagNumber(2)
  $core.String get detail => $_getSZ(1);
  @$pb.TagNumber(2)
  set detail($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasDetail() => $_has(1);
  @$pb.TagNumber(2)
  void clearDetail() => $_clearField(2);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
