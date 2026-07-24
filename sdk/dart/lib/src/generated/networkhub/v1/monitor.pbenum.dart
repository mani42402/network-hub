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

import 'package:protobuf/protobuf.dart' as $pb;

class InterfaceKind extends $pb.ProtobufEnum {
  static const InterfaceKind INTERFACE_KIND_UNSPECIFIED =
      InterfaceKind._(0, _omitEnumNames ? '' : 'INTERFACE_KIND_UNSPECIFIED');
  static const InterfaceKind INTERFACE_KIND_ETHERNET =
      InterfaceKind._(1, _omitEnumNames ? '' : 'INTERFACE_KIND_ETHERNET');
  static const InterfaceKind INTERFACE_KIND_WIFI =
      InterfaceKind._(2, _omitEnumNames ? '' : 'INTERFACE_KIND_WIFI');
  static const InterfaceKind INTERFACE_KIND_LOOPBACK =
      InterfaceKind._(3, _omitEnumNames ? '' : 'INTERFACE_KIND_LOOPBACK');

  /// A VPN tunnel interface (utun/wintun/wg).
  static const InterfaceKind INTERFACE_KIND_TUNNEL =
      InterfaceKind._(4, _omitEnumNames ? '' : 'INTERFACE_KIND_TUNNEL');

  /// A virtual/bridge interface, e.g. a hotspot bridge.
  static const InterfaceKind INTERFACE_KIND_VIRTUAL =
      InterfaceKind._(5, _omitEnumNames ? '' : 'INTERFACE_KIND_VIRTUAL');
  static const InterfaceKind INTERFACE_KIND_OTHER =
      InterfaceKind._(6, _omitEnumNames ? '' : 'INTERFACE_KIND_OTHER');

  static const $core.List<InterfaceKind> values = <InterfaceKind>[
    INTERFACE_KIND_UNSPECIFIED,
    INTERFACE_KIND_ETHERNET,
    INTERFACE_KIND_WIFI,
    INTERFACE_KIND_LOOPBACK,
    INTERFACE_KIND_TUNNEL,
    INTERFACE_KIND_VIRTUAL,
    INTERFACE_KIND_OTHER,
  ];

  static final $core.List<InterfaceKind?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 6);
  static InterfaceKind? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const InterfaceKind._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
