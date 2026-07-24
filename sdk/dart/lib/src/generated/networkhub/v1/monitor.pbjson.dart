// This is a generated file - do not edit.
//
// Generated from networkhub/v1/monitor.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports
// ignore_for_file: unused_import

import 'dart:convert' as $convert;
import 'dart:core' as $core;
import 'dart:typed_data' as $typed_data;

@$core.Deprecated('Use interfaceKindDescriptor instead')
const InterfaceKind$json = {
  '1': 'InterfaceKind',
  '2': [
    {'1': 'INTERFACE_KIND_UNSPECIFIED', '2': 0},
    {'1': 'INTERFACE_KIND_ETHERNET', '2': 1},
    {'1': 'INTERFACE_KIND_WIFI', '2': 2},
    {'1': 'INTERFACE_KIND_LOOPBACK', '2': 3},
    {'1': 'INTERFACE_KIND_TUNNEL', '2': 4},
    {'1': 'INTERFACE_KIND_VIRTUAL', '2': 5},
    {'1': 'INTERFACE_KIND_OTHER', '2': 6},
  ],
};

/// Descriptor for `InterfaceKind`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List interfaceKindDescriptor = $convert.base64Decode(
    'Cg1JbnRlcmZhY2VLaW5kEh4KGklOVEVSRkFDRV9LSU5EX1VOU1BFQ0lGSUVEEAASGwoXSU5URV'
    'JGQUNFX0tJTkRfRVRIRVJORVQQARIXChNJTlRFUkZBQ0VfS0lORF9XSUZJEAISGwoXSU5URVJG'
    'QUNFX0tJTkRfTE9PUEJBQ0sQAxIZChVJTlRFUkZBQ0VfS0lORF9UVU5ORUwQBBIaChZJTlRFUk'
    'ZBQ0VfS0lORF9WSVJUVUFMEAUSGAoUSU5URVJGQUNFX0tJTkRfT1RIRVIQBg==');

@$core.Deprecated('Use listInterfacesRequestDescriptor instead')
const ListInterfacesRequest$json = {
  '1': 'ListInterfacesRequest',
};

/// Descriptor for `ListInterfacesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listInterfacesRequestDescriptor =
    $convert.base64Decode('ChVMaXN0SW50ZXJmYWNlc1JlcXVlc3Q=');

@$core.Deprecated('Use listInterfacesResponseDescriptor instead')
const ListInterfacesResponse$json = {
  '1': 'ListInterfacesResponse',
  '2': [
    {
      '1': 'interfaces',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.Interface',
      '10': 'interfaces'
    },
  ],
};

/// Descriptor for `ListInterfacesResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listInterfacesResponseDescriptor =
    $convert.base64Decode(
        'ChZMaXN0SW50ZXJmYWNlc1Jlc3BvbnNlEjgKCmludGVyZmFjZXMYASADKAsyGC5uZXR3b3JraH'
        'ViLnYxLkludGVyZmFjZVIKaW50ZXJmYWNlcw==');

@$core.Deprecated('Use interfaceDescriptor instead')
const Interface$json = {
  '1': 'Interface',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {'1': 'name', '3': 2, '4': 1, '5': 9, '10': 'name'},
    {
      '1': 'kind',
      '3': 3,
      '4': 1,
      '5': 14,
      '6': '.networkhub.v1.InterfaceKind',
      '10': 'kind'
    },
    {'1': 'is_up', '3': 4, '4': 1, '5': 8, '10': 'isUp'},
    {'1': 'mac', '3': 5, '4': 1, '5': 9, '10': 'mac'},
    {'1': 'addresses', '3': 6, '4': 3, '5': 9, '10': 'addresses'},
  ],
};

/// Descriptor for `Interface`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List interfaceDescriptor = $convert.base64Decode(
    'CglJbnRlcmZhY2USDgoCaWQYASABKAlSAmlkEhIKBG5hbWUYAiABKAlSBG5hbWUSMAoEa2luZB'
    'gDIAEoDjIcLm5ldHdvcmtodWIudjEuSW50ZXJmYWNlS2luZFIEa2luZBITCgVpc191cBgEIAEo'
    'CFIEaXNVcBIQCgNtYWMYBSABKAlSA21hYxIcCglhZGRyZXNzZXMYBiADKAlSCWFkZHJlc3Nlcw'
    '==');

@$core.Deprecated('Use streamThroughputRequestDescriptor instead')
const StreamThroughputRequest$json = {
  '1': 'StreamThroughputRequest',
  '2': [
    {'1': 'interface_ids', '3': 1, '4': 3, '5': 9, '10': 'interfaceIds'},
  ],
};

/// Descriptor for `StreamThroughputRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List streamThroughputRequestDescriptor =
    $convert.base64Decode(
        'ChdTdHJlYW1UaHJvdWdocHV0UmVxdWVzdBIjCg1pbnRlcmZhY2VfaWRzGAEgAygJUgxpbnRlcm'
        'ZhY2VJZHM=');

@$core.Deprecated('Use throughputSampleDescriptor instead')
const ThroughputSample$json = {
  '1': 'ThroughputSample',
  '2': [
    {'1': 'unix_millis', '3': 1, '4': 1, '5': 3, '10': 'unixMillis'},
    {
      '1': 'interfaces',
      '3': 2,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.InterfaceThroughput',
      '10': 'interfaces'
    },
  ],
};

/// Descriptor for `ThroughputSample`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List throughputSampleDescriptor = $convert.base64Decode(
    'ChBUaHJvdWdocHV0U2FtcGxlEh8KC3VuaXhfbWlsbGlzGAEgASgDUgp1bml4TWlsbGlzEkIKCm'
    'ludGVyZmFjZXMYAiADKAsyIi5uZXR3b3JraHViLnYxLkludGVyZmFjZVRocm91Z2hwdXRSCmlu'
    'dGVyZmFjZXM=');

@$core.Deprecated('Use interfaceThroughputDescriptor instead')
const InterfaceThroughput$json = {
  '1': 'InterfaceThroughput',
  '2': [
    {'1': 'interface_id', '3': 1, '4': 1, '5': 9, '10': 'interfaceId'},
    {
      '1': 'rx',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.BitRate',
      '10': 'rx'
    },
    {
      '1': 'tx',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.BitRate',
      '10': 'tx'
    },
    {
      '1': 'rx_total',
      '3': 4,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.ByteCount',
      '10': 'rxTotal'
    },
    {
      '1': 'tx_total',
      '3': 5,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.ByteCount',
      '10': 'txTotal'
    },
  ],
};

/// Descriptor for `InterfaceThroughput`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List interfaceThroughputDescriptor = $convert.base64Decode(
    'ChNJbnRlcmZhY2VUaHJvdWdocHV0EiEKDGludGVyZmFjZV9pZBgBIAEoCVILaW50ZXJmYWNlSW'
    'QSJgoCcngYAiABKAsyFi5uZXR3b3JraHViLnYxLkJpdFJhdGVSAnJ4EiYKAnR4GAMgASgLMhYu'
    'bmV0d29ya2h1Yi52MS5CaXRSYXRlUgJ0eBIzCghyeF90b3RhbBgEIAEoCzIYLm5ldHdvcmtodW'
    'IudjEuQnl0ZUNvdW50UgdyeFRvdGFsEjMKCHR4X3RvdGFsGAUgASgLMhgubmV0d29ya2h1Yi52'
    'MS5CeXRlQ291bnRSB3R4VG90YWw=');
