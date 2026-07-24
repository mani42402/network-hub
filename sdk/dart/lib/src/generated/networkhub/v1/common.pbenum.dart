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

import 'package:protobuf/protobuf.dart' as $pb;

/// The set of permissions a principal can hold. Coarser than one-per-RPC on
/// purpose: roles grant permissions, and a readable matrix beats a sprawling
/// list. Read permissions gate observing state; write permissions gate changing
/// it; the ADMIN_* permissions gate security-sensitive operations that also
/// require recent re-auth on remote sessions (§10.3).
class Permission extends $pb.ProtobufEnum {
  /// Never valid as a requirement. Its presence on an RPC is a bug the
  /// annotation-coverage test rejects, so a zero-value default cannot slip
  /// through as "no permission needed".
  static const Permission PERMISSION_UNSPECIFIED =
      Permission._(0, _omitEnumNames ? '' : 'PERMISSION_UNSPECIFIED');

  /// Read-only observation of non-sensitive state: capabilities, health,
  /// interfaces, live metrics. The default for a viewer.
  static const Permission PERMISSION_SYSTEM_READ =
      Permission._(1, _omitEnumNames ? '' : 'PERMISSION_SYSTEM_READ');

  /// Read device inventory and status.
  static const Permission PERMISSION_DEVICE_READ =
      Permission._(2, _omitEnumNames ? '' : 'PERMISSION_DEVICE_READ');

  /// Change device state: block, pause, rename, set quotas.
  static const Permission PERMISSION_DEVICE_WRITE =
      Permission._(3, _omitEnumNames ? '' : 'PERMISSION_DEVICE_WRITE');

  /// Read VPN profiles and status.
  static const Permission PERMISSION_VPN_READ =
      Permission._(4, _omitEnumNames ? '' : 'PERMISSION_VPN_READ');

  /// Manage VPN profiles and connections.
  static const Permission PERMISSION_VPN_WRITE =
      Permission._(5, _omitEnumNames ? '' : 'PERMISSION_VPN_WRITE');

  /// Read hotspot profiles and status.
  static const Permission PERMISSION_HOTSPOT_READ =
      Permission._(6, _omitEnumNames ? '' : 'PERMISSION_HOTSPOT_READ');

  /// Manage hotspot profiles and lifecycle.
  static const Permission PERMISSION_HOTSPOT_WRITE =
      Permission._(7, _omitEnumNames ? '' : 'PERMISSION_HOTSPOT_WRITE');

  /// Read firewall rules.
  static const Permission PERMISSION_FIREWALL_READ =
      Permission._(8, _omitEnumNames ? '' : 'PERMISSION_FIREWALL_READ');

  /// Change firewall rules. Security-sensitive.
  static const Permission PERMISSION_FIREWALL_WRITE =
      Permission._(9, _omitEnumNames ? '' : 'PERMISSION_FIREWALL_WRITE');

  /// Administer security-sensitive settings: enable the gateway, install
  /// plugins, export secrets, set a persistent kill switch. Requires recent
  /// re-auth on remote sessions (§10.3).
  static const Permission PERMISSION_ADMIN =
      Permission._(10, _omitEnumNames ? '' : 'PERMISSION_ADMIN');

  static const $core.List<Permission> values = <Permission>[
    PERMISSION_UNSPECIFIED,
    PERMISSION_SYSTEM_READ,
    PERMISSION_DEVICE_READ,
    PERMISSION_DEVICE_WRITE,
    PERMISSION_VPN_READ,
    PERMISSION_VPN_WRITE,
    PERMISSION_HOTSPOT_READ,
    PERMISSION_HOTSPOT_WRITE,
    PERMISSION_FIREWALL_READ,
    PERMISSION_FIREWALL_WRITE,
    PERMISSION_ADMIN,
  ];

  static final $core.List<Permission?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 10);
  static Permission? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const Permission._(super.value, super.name);
}

/// Overall serving state of the daemon or one of its modules. Shared by
/// SystemService.GetHealth and the event stream's health events.
class ServingStatus extends $pb.ProtobufEnum {
  static const ServingStatus SERVING_STATUS_UNSPECIFIED =
      ServingStatus._(0, _omitEnumNames ? '' : 'SERVING_STATUS_UNSPECIFIED');

  /// Fully operational.
  static const ServingStatus SERVING_STATUS_SERVING =
      ServingStatus._(1, _omitEnumNames ? '' : 'SERVING_STATUS_SERVING');

  /// Running but a module is impaired; details accompany it.
  static const ServingStatus SERVING_STATUS_DEGRADED =
      ServingStatus._(2, _omitEnumNames ? '' : 'SERVING_STATUS_DEGRADED');

  /// Not able to serve.
  static const ServingStatus SERVING_STATUS_NOT_SERVING =
      ServingStatus._(3, _omitEnumNames ? '' : 'SERVING_STATUS_NOT_SERVING');

  static const $core.List<ServingStatus> values = <ServingStatus>[
    SERVING_STATUS_UNSPECIFIED,
    SERVING_STATUS_SERVING,
    SERVING_STATUS_DEGRADED,
    SERVING_STATUS_NOT_SERVING,
  ];

  static final $core.List<ServingStatus?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 3);
  static ServingStatus? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const ServingStatus._(super.value, super.name);
}

/// A capability's usability (AD-6). Mirrors nh_types::CapabilityStatus.
class CapabilityStatus extends $pb.ProtobufEnum {
  static const CapabilityStatus CAPABILITY_STATUS_UNSPECIFIED =
      CapabilityStatus._(
          0, _omitEnumNames ? '' : 'CAPABILITY_STATUS_UNSPECIFIED');

  /// Fully usable on this machine.
  static const CapabilityStatus CAPABILITY_STATUS_SUPPORTED =
      CapabilityStatus._(
          1, _omitEnumNames ? '' : 'CAPABILITY_STATUS_SUPPORTED');

  /// Usable with limitations the reason/details explain.
  static const CapabilityStatus CAPABILITY_STATUS_DEGRADED =
      CapabilityStatus._(2, _omitEnumNames ? '' : 'CAPABILITY_STATUS_DEGRADED');

  /// Not usable here; the reason says why.
  static const CapabilityStatus CAPABILITY_STATUS_UNSUPPORTED =
      CapabilityStatus._(
          3, _omitEnumNames ? '' : 'CAPABILITY_STATUS_UNSUPPORTED');

  /// Not yet probed; distinct from unsupported.
  static const CapabilityStatus CAPABILITY_STATUS_UNKNOWN =
      CapabilityStatus._(4, _omitEnumNames ? '' : 'CAPABILITY_STATUS_UNKNOWN');

  static const $core.List<CapabilityStatus> values = <CapabilityStatus>[
    CAPABILITY_STATUS_UNSPECIFIED,
    CAPABILITY_STATUS_SUPPORTED,
    CAPABILITY_STATUS_DEGRADED,
    CAPABILITY_STATUS_UNSUPPORTED,
    CAPABILITY_STATUS_UNKNOWN,
  ];

  static final $core.List<CapabilityStatus?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 4);
  static CapabilityStatus? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const CapabilityStatus._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
