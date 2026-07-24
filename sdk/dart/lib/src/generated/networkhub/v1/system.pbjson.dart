// This is a generated file - do not edit.
//
// Generated from networkhub/v1/system.proto.

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

@$core.Deprecated('Use getVersionRequestDescriptor instead')
const GetVersionRequest$json = {
  '1': 'GetVersionRequest',
};

/// Descriptor for `GetVersionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getVersionRequestDescriptor =
    $convert.base64Decode('ChFHZXRWZXJzaW9uUmVxdWVzdA==');

@$core.Deprecated('Use getVersionResponseDescriptor instead')
const GetVersionResponse$json = {
  '1': 'GetVersionResponse',
  '2': [
    {'1': 'daemon_version', '3': 1, '4': 1, '5': 9, '10': 'daemonVersion'},
    {'1': 'api_version', '3': 2, '4': 1, '5': 9, '10': 'apiVersion'},
    {
      '1': 'min_client_version',
      '3': 3,
      '4': 1,
      '5': 9,
      '10': 'minClientVersion'
    },
    {'1': 'os', '3': 4, '4': 1, '5': 9, '10': 'os'},
  ],
};

/// Descriptor for `GetVersionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getVersionResponseDescriptor = $convert.base64Decode(
    'ChJHZXRWZXJzaW9uUmVzcG9uc2USJQoOZGFlbW9uX3ZlcnNpb24YASABKAlSDWRhZW1vblZlcn'
    'Npb24SHwoLYXBpX3ZlcnNpb24YAiABKAlSCmFwaVZlcnNpb24SLAoSbWluX2NsaWVudF92ZXJz'
    'aW9uGAMgASgJUhBtaW5DbGllbnRWZXJzaW9uEg4KAm9zGAQgASgJUgJvcw==');

@$core.Deprecated('Use getHealthRequestDescriptor instead')
const GetHealthRequest$json = {
  '1': 'GetHealthRequest',
};

/// Descriptor for `GetHealthRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getHealthRequestDescriptor =
    $convert.base64Decode('ChBHZXRIZWFsdGhSZXF1ZXN0');

@$core.Deprecated('Use getHealthResponseDescriptor instead')
const GetHealthResponse$json = {
  '1': 'GetHealthResponse',
  '2': [
    {
      '1': 'overall',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.networkhub.v1.ServingStatus',
      '10': 'overall'
    },
    {
      '1': 'modules',
      '3': 2,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.ModuleHealth',
      '10': 'modules'
    },
  ],
};

/// Descriptor for `GetHealthResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getHealthResponseDescriptor = $convert.base64Decode(
    'ChFHZXRIZWFsdGhSZXNwb25zZRI2CgdvdmVyYWxsGAEgASgOMhwubmV0d29ya2h1Yi52MS5TZX'
    'J2aW5nU3RhdHVzUgdvdmVyYWxsEjUKB21vZHVsZXMYAiADKAsyGy5uZXR3b3JraHViLnYxLk1v'
    'ZHVsZUhlYWx0aFIHbW9kdWxlcw==');

@$core.Deprecated('Use moduleHealthDescriptor instead')
const ModuleHealth$json = {
  '1': 'ModuleHealth',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {
      '1': 'status',
      '3': 2,
      '4': 1,
      '5': 14,
      '6': '.networkhub.v1.ServingStatus',
      '10': 'status'
    },
    {'1': 'detail', '3': 3, '4': 1, '5': 9, '10': 'detail'},
  ],
};

/// Descriptor for `ModuleHealth`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List moduleHealthDescriptor = $convert.base64Decode(
    'CgxNb2R1bGVIZWFsdGgSEgoEbmFtZRgBIAEoCVIEbmFtZRI0CgZzdGF0dXMYAiABKA4yHC5uZX'
    'R3b3JraHViLnYxLlNlcnZpbmdTdGF0dXNSBnN0YXR1cxIWCgZkZXRhaWwYAyABKAlSBmRldGFp'
    'bA==');

@$core.Deprecated('Use getCapabilitiesRequestDescriptor instead')
const GetCapabilitiesRequest$json = {
  '1': 'GetCapabilitiesRequest',
};

/// Descriptor for `GetCapabilitiesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getCapabilitiesRequestDescriptor =
    $convert.base64Decode('ChZHZXRDYXBhYmlsaXRpZXNSZXF1ZXN0');

@$core.Deprecated('Use getCapabilitiesResponseDescriptor instead')
const GetCapabilitiesResponse$json = {
  '1': 'GetCapabilitiesResponse',
  '2': [
    {
      '1': 'capabilities',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.GetCapabilitiesResponse.CapabilitiesEntry',
      '10': 'capabilities'
    },
  ],
  '3': [GetCapabilitiesResponse_CapabilitiesEntry$json],
};

@$core.Deprecated('Use getCapabilitiesResponseDescriptor instead')
const GetCapabilitiesResponse_CapabilitiesEntry$json = {
  '1': 'CapabilitiesEntry',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {
      '1': 'value',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.Capability',
      '10': 'value'
    },
  ],
  '7': {'7': true},
};

/// Descriptor for `GetCapabilitiesResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getCapabilitiesResponseDescriptor = $convert.base64Decode(
    'ChdHZXRDYXBhYmlsaXRpZXNSZXNwb25zZRJcCgxjYXBhYmlsaXRpZXMYASADKAsyOC5uZXR3b3'
    'JraHViLnYxLkdldENhcGFiaWxpdGllc1Jlc3BvbnNlLkNhcGFiaWxpdGllc0VudHJ5UgxjYXBh'
    'YmlsaXRpZXMaWgoRQ2FwYWJpbGl0aWVzRW50cnkSEAoDa2V5GAEgASgJUgNrZXkSLwoFdmFsdW'
    'UYAiABKAsyGS5uZXR3b3JraHViLnYxLkNhcGFiaWxpdHlSBXZhbHVlOgI4AQ==');
