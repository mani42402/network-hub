// This is a generated file - do not edit.
//
// Generated from networkhub/v1/common.proto.

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

@$core.Deprecated('Use permissionDescriptor instead')
const Permission$json = {
  '1': 'Permission',
  '2': [
    {'1': 'PERMISSION_UNSPECIFIED', '2': 0},
    {'1': 'PERMISSION_SYSTEM_READ', '2': 1},
    {'1': 'PERMISSION_DEVICE_READ', '2': 2},
    {'1': 'PERMISSION_DEVICE_WRITE', '2': 3},
    {'1': 'PERMISSION_VPN_READ', '2': 4},
    {'1': 'PERMISSION_VPN_WRITE', '2': 5},
    {'1': 'PERMISSION_HOTSPOT_READ', '2': 6},
    {'1': 'PERMISSION_HOTSPOT_WRITE', '2': 7},
    {'1': 'PERMISSION_FIREWALL_READ', '2': 8},
    {'1': 'PERMISSION_FIREWALL_WRITE', '2': 9},
    {'1': 'PERMISSION_ADMIN', '2': 10},
  ],
};

/// Descriptor for `Permission`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List permissionDescriptor = $convert.base64Decode(
    'CgpQZXJtaXNzaW9uEhoKFlBFUk1JU1NJT05fVU5TUEVDSUZJRUQQABIaChZQRVJNSVNTSU9OX1'
    'NZU1RFTV9SRUFEEAESGgoWUEVSTUlTU0lPTl9ERVZJQ0VfUkVBRBACEhsKF1BFUk1JU1NJT05f'
    'REVWSUNFX1dSSVRFEAMSFwoTUEVSTUlTU0lPTl9WUE5fUkVBRBAEEhgKFFBFUk1JU1NJT05fVl'
    'BOX1dSSVRFEAUSGwoXUEVSTUlTU0lPTl9IT1RTUE9UX1JFQUQQBhIcChhQRVJNSVNTSU9OX0hP'
    'VFNQT1RfV1JJVEUQBxIcChhQRVJNSVNTSU9OX0ZJUkVXQUxMX1JFQUQQCBIdChlQRVJNSVNTSU'
    '9OX0ZJUkVXQUxMX1dSSVRFEAkSFAoQUEVSTUlTU0lPTl9BRE1JThAK');

@$core.Deprecated('Use servingStatusDescriptor instead')
const ServingStatus$json = {
  '1': 'ServingStatus',
  '2': [
    {'1': 'SERVING_STATUS_UNSPECIFIED', '2': 0},
    {'1': 'SERVING_STATUS_SERVING', '2': 1},
    {'1': 'SERVING_STATUS_DEGRADED', '2': 2},
    {'1': 'SERVING_STATUS_NOT_SERVING', '2': 3},
  ],
};

/// Descriptor for `ServingStatus`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List servingStatusDescriptor = $convert.base64Decode(
    'Cg1TZXJ2aW5nU3RhdHVzEh4KGlNFUlZJTkdfU1RBVFVTX1VOU1BFQ0lGSUVEEAASGgoWU0VSVk'
    'lOR19TVEFUVVNfU0VSVklORxABEhsKF1NFUlZJTkdfU1RBVFVTX0RFR1JBREVEEAISHgoaU0VS'
    'VklOR19TVEFUVVNfTk9UX1NFUlZJTkcQAw==');

@$core.Deprecated('Use capabilityStatusDescriptor instead')
const CapabilityStatus$json = {
  '1': 'CapabilityStatus',
  '2': [
    {'1': 'CAPABILITY_STATUS_UNSPECIFIED', '2': 0},
    {'1': 'CAPABILITY_STATUS_SUPPORTED', '2': 1},
    {'1': 'CAPABILITY_STATUS_DEGRADED', '2': 2},
    {'1': 'CAPABILITY_STATUS_UNSUPPORTED', '2': 3},
    {'1': 'CAPABILITY_STATUS_UNKNOWN', '2': 4},
  ],
};

/// Descriptor for `CapabilityStatus`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List capabilityStatusDescriptor = $convert.base64Decode(
    'ChBDYXBhYmlsaXR5U3RhdHVzEiEKHUNBUEFCSUxJVFlfU1RBVFVTX1VOU1BFQ0lGSUVEEAASHw'
    'obQ0FQQUJJTElUWV9TVEFUVVNfU1VQUE9SVEVEEAESHgoaQ0FQQUJJTElUWV9TVEFUVVNfREVH'
    'UkFERUQQAhIhCh1DQVBBQklMSVRZX1NUQVRVU19VTlNVUFBPUlRFRBADEh0KGUNBUEFCSUxJVF'
    'lfU1RBVFVTX1VOS05PV04QBA==');

@$core.Deprecated('Use capabilityDescriptor instead')
const Capability$json = {
  '1': 'Capability',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.networkhub.v1.CapabilityStatus',
      '10': 'status'
    },
    {'1': 'reason', '3': 2, '4': 1, '5': 9, '10': 'reason'},
    {
      '1': 'details',
      '3': 3,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.Capability.DetailsEntry',
      '10': 'details'
    },
  ],
  '3': [Capability_DetailsEntry$json],
};

@$core.Deprecated('Use capabilityDescriptor instead')
const Capability_DetailsEntry$json = {
  '1': 'DetailsEntry',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'value', '3': 2, '4': 1, '5': 9, '10': 'value'},
  ],
  '7': {'7': true},
};

/// Descriptor for `Capability`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List capabilityDescriptor = $convert.base64Decode(
    'CgpDYXBhYmlsaXR5EjcKBnN0YXR1cxgBIAEoDjIfLm5ldHdvcmtodWIudjEuQ2FwYWJpbGl0eV'
    'N0YXR1c1IGc3RhdHVzEhYKBnJlYXNvbhgCIAEoCVIGcmVhc29uEkAKB2RldGFpbHMYAyADKAsy'
    'Ji5uZXR3b3JraHViLnYxLkNhcGFiaWxpdHkuRGV0YWlsc0VudHJ5UgdkZXRhaWxzGjoKDERldG'
    'FpbHNFbnRyeRIQCgNrZXkYASABKAlSA2tleRIUCgV2YWx1ZRgCIAEoCVIFdmFsdWU6AjgB');

@$core.Deprecated('Use errorInfoDescriptor instead')
const ErrorInfo$json = {
  '1': 'ErrorInfo',
  '2': [
    {'1': 'reason', '3': 1, '4': 1, '5': 9, '10': 'reason'},
    {'1': 'detail', '3': 2, '4': 1, '5': 9, '10': 'detail'},
    {'1': 'remediation', '3': 3, '4': 1, '5': 9, '10': 'remediation'},
  ],
};

/// Descriptor for `ErrorInfo`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List errorInfoDescriptor = $convert.base64Decode(
    'CglFcnJvckluZm8SFgoGcmVhc29uGAEgASgJUgZyZWFzb24SFgoGZGV0YWlsGAIgASgJUgZkZX'
    'RhaWwSIAoLcmVtZWRpYXRpb24YAyABKAlSC3JlbWVkaWF0aW9u');

@$core.Deprecated('Use byteCountDescriptor instead')
const ByteCount$json = {
  '1': 'ByteCount',
  '2': [
    {'1': 'bytes', '3': 1, '4': 1, '5': 4, '10': 'bytes'},
  ],
};

/// Descriptor for `ByteCount`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List byteCountDescriptor =
    $convert.base64Decode('CglCeXRlQ291bnQSFAoFYnl0ZXMYASABKARSBWJ5dGVz');

@$core.Deprecated('Use bitRateDescriptor instead')
const BitRate$json = {
  '1': 'BitRate',
  '2': [
    {'1': 'bits_per_sec', '3': 1, '4': 1, '5': 4, '10': 'bitsPerSec'},
  ],
};

/// Descriptor for `BitRate`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List bitRateDescriptor = $convert.base64Decode(
    'CgdCaXRSYXRlEiAKDGJpdHNfcGVyX3NlYxgBIAEoBFIKYml0c1BlclNlYw==');
