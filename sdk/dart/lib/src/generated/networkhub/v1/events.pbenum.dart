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

import 'package:protobuf/protobuf.dart' as $pb;

/// The categories of event a client can subscribe to. A subscription is filtered
/// to the caller's permissions regardless of what is requested.
class EventTopic extends $pb.ProtobufEnum {
  static const EventTopic EVENT_TOPIC_UNSPECIFIED =
      EventTopic._(0, _omitEnumNames ? '' : 'EVENT_TOPIC_UNSPECIFIED');

  /// Device joined/left/changed.
  static const EventTopic EVENT_TOPIC_DEVICES =
      EventTopic._(1, _omitEnumNames ? '' : 'EVENT_TOPIC_DEVICES');

  /// VPN connection state transitions.
  static const EventTopic EVENT_TOPIC_VPN =
      EventTopic._(2, _omitEnumNames ? '' : 'EVENT_TOPIC_VPN');

  /// Hotspot lifecycle transitions.
  static const EventTopic EVENT_TOPIC_HOTSPOT =
      EventTopic._(3, _omitEnumNames ? '' : 'EVENT_TOPIC_HOTSPOT');

  /// Capability matrix changed (hardware added/removed).
  static const EventTopic EVENT_TOPIC_CAPABILITIES =
      EventTopic._(4, _omitEnumNames ? '' : 'EVENT_TOPIC_CAPABILITIES');

  /// A module changed health (e.g. entered degraded/conflicted).
  static const EventTopic EVENT_TOPIC_HEALTH =
      EventTopic._(5, _omitEnumNames ? '' : 'EVENT_TOPIC_HEALTH');

  /// A conflict was detected with another network agent (§5.3).
  static const EventTopic EVENT_TOPIC_CONFLICTS =
      EventTopic._(6, _omitEnumNames ? '' : 'EVENT_TOPIC_CONFLICTS');

  static const $core.List<EventTopic> values = <EventTopic>[
    EVENT_TOPIC_UNSPECIFIED,
    EVENT_TOPIC_DEVICES,
    EVENT_TOPIC_VPN,
    EVENT_TOPIC_HOTSPOT,
    EVENT_TOPIC_CAPABILITIES,
    EVENT_TOPIC_HEALTH,
    EVENT_TOPIC_CONFLICTS,
  ];

  static final $core.List<EventTopic?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 6);
  static EventTopic? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const EventTopic._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
