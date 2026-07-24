// This is a generated file - do not edit.
//
// Generated from networkhub/v1/events.proto.

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

@$core.Deprecated('Use eventTopicDescriptor instead')
const EventTopic$json = {
  '1': 'EventTopic',
  '2': [
    {'1': 'EVENT_TOPIC_UNSPECIFIED', '2': 0},
    {'1': 'EVENT_TOPIC_DEVICES', '2': 1},
    {'1': 'EVENT_TOPIC_VPN', '2': 2},
    {'1': 'EVENT_TOPIC_HOTSPOT', '2': 3},
    {'1': 'EVENT_TOPIC_CAPABILITIES', '2': 4},
    {'1': 'EVENT_TOPIC_HEALTH', '2': 5},
    {'1': 'EVENT_TOPIC_CONFLICTS', '2': 6},
  ],
};

/// Descriptor for `EventTopic`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List eventTopicDescriptor = $convert.base64Decode(
    'CgpFdmVudFRvcGljEhsKF0VWRU5UX1RPUElDX1VOU1BFQ0lGSUVEEAASFwoTRVZFTlRfVE9QSU'
    'NfREVWSUNFUxABEhMKD0VWRU5UX1RPUElDX1ZQThACEhcKE0VWRU5UX1RPUElDX0hPVFNQT1QQ'
    'AxIcChhFVkVOVF9UT1BJQ19DQVBBQklMSVRJRVMQBBIWChJFVkVOVF9UT1BJQ19IRUFMVEgQBR'
    'IZChVFVkVOVF9UT1BJQ19DT05GTElDVFMQBg==');

@$core.Deprecated('Use subscribeRequestDescriptor instead')
const SubscribeRequest$json = {
  '1': 'SubscribeRequest',
  '2': [
    {
      '1': 'topics',
      '3': 1,
      '4': 3,
      '5': 14,
      '6': '.networkhub.v1.EventTopic',
      '10': 'topics'
    },
  ],
};

/// Descriptor for `SubscribeRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List subscribeRequestDescriptor = $convert.base64Decode(
    'ChBTdWJzY3JpYmVSZXF1ZXN0EjEKBnRvcGljcxgBIAMoDjIZLm5ldHdvcmtodWIudjEuRXZlbn'
    'RUb3BpY1IGdG9waWNz');

@$core.Deprecated('Use streamMessageDescriptor instead')
const StreamMessage$json = {
  '1': 'StreamMessage',
  '2': [
    {'1': 'sequence', '3': 1, '4': 1, '5': 4, '10': 'sequence'},
    {
      '1': 'snapshot',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.Snapshot',
      '9': 0,
      '10': 'snapshot'
    },
    {
      '1': 'delta',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.Delta',
      '9': 0,
      '10': 'delta'
    },
  ],
  '8': [
    {'1': 'payload'},
  ],
};

/// Descriptor for `StreamMessage`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List streamMessageDescriptor = $convert.base64Decode(
    'Cg1TdHJlYW1NZXNzYWdlEhoKCHNlcXVlbmNlGAEgASgEUghzZXF1ZW5jZRI1CghzbmFwc2hvdB'
    'gCIAEoCzIXLm5ldHdvcmtodWIudjEuU25hcHNob3RIAFIIc25hcHNob3QSLAoFZGVsdGEYAyAB'
    'KAsyFC5uZXR3b3JraHViLnYxLkRlbHRhSABSBWRlbHRhQgkKB3BheWxvYWQ=');

@$core.Deprecated('Use snapshotDescriptor instead')
const Snapshot$json = {
  '1': 'Snapshot',
  '2': [
    {
      '1': 'capabilities',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.Snapshot.CapabilitiesEntry',
      '10': 'capabilities'
    },
    {
      '1': 'module_health',
      '3': 2,
      '4': 3,
      '5': 11,
      '6': '.networkhub.v1.ModuleHealthEvent',
      '10': 'moduleHealth'
    },
  ],
  '3': [Snapshot_CapabilitiesEntry$json],
};

@$core.Deprecated('Use snapshotDescriptor instead')
const Snapshot_CapabilitiesEntry$json = {
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

/// Descriptor for `Snapshot`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List snapshotDescriptor = $convert.base64Decode(
    'CghTbmFwc2hvdBJNCgxjYXBhYmlsaXRpZXMYASADKAsyKS5uZXR3b3JraHViLnYxLlNuYXBzaG'
    '90LkNhcGFiaWxpdGllc0VudHJ5UgxjYXBhYmlsaXRpZXMSRQoNbW9kdWxlX2hlYWx0aBgCIAMo'
    'CzIgLm5ldHdvcmtodWIudjEuTW9kdWxlSGVhbHRoRXZlbnRSDG1vZHVsZUhlYWx0aBpaChFDYX'
    'BhYmlsaXRpZXNFbnRyeRIQCgNrZXkYASABKAlSA2tleRIvCgV2YWx1ZRgCIAEoCzIZLm5ldHdv'
    'cmtodWIudjEuQ2FwYWJpbGl0eVIFdmFsdWU6AjgB');

@$core.Deprecated('Use deltaDescriptor instead')
const Delta$json = {
  '1': 'Delta',
  '2': [
    {
      '1': 'capability_changed',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.CapabilityChanged',
      '9': 0,
      '10': 'capabilityChanged'
    },
    {
      '1': 'module_health_changed',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.ModuleHealthEvent',
      '9': 0,
      '10': 'moduleHealthChanged'
    },
    {
      '1': 'conflict_detected',
      '3': 3,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.ConflictDetected',
      '9': 0,
      '10': 'conflictDetected'
    },
  ],
  '8': [
    {'1': 'event'},
  ],
};

/// Descriptor for `Delta`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List deltaDescriptor = $convert.base64Decode(
    'CgVEZWx0YRJRChJjYXBhYmlsaXR5X2NoYW5nZWQYASABKAsyIC5uZXR3b3JraHViLnYxLkNhcG'
    'FiaWxpdHlDaGFuZ2VkSABSEWNhcGFiaWxpdHlDaGFuZ2VkElYKFW1vZHVsZV9oZWFsdGhfY2hh'
    'bmdlZBgCIAEoCzIgLm5ldHdvcmtodWIudjEuTW9kdWxlSGVhbHRoRXZlbnRIAFITbW9kdWxlSG'
    'VhbHRoQ2hhbmdlZBJOChFjb25mbGljdF9kZXRlY3RlZBgDIAEoCzIfLm5ldHdvcmtodWIudjEu'
    'Q29uZmxpY3REZXRlY3RlZEgAUhBjb25mbGljdERldGVjdGVkQgcKBWV2ZW50');

@$core.Deprecated('Use capabilityChangedDescriptor instead')
const CapabilityChanged$json = {
  '1': 'CapabilityChanged',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {
      '1': 'capability',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.networkhub.v1.Capability',
      '10': 'capability'
    },
  ],
};

/// Descriptor for `CapabilityChanged`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List capabilityChangedDescriptor = $convert.base64Decode(
    'ChFDYXBhYmlsaXR5Q2hhbmdlZBIQCgNrZXkYASABKAlSA2tleRI5CgpjYXBhYmlsaXR5GAIgAS'
    'gLMhkubmV0d29ya2h1Yi52MS5DYXBhYmlsaXR5UgpjYXBhYmlsaXR5');

@$core.Deprecated('Use moduleHealthEventDescriptor instead')
const ModuleHealthEvent$json = {
  '1': 'ModuleHealthEvent',
  '2': [
    {'1': 'module', '3': 1, '4': 1, '5': 9, '10': 'module'},
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

/// Descriptor for `ModuleHealthEvent`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List moduleHealthEventDescriptor = $convert.base64Decode(
    'ChFNb2R1bGVIZWFsdGhFdmVudBIWCgZtb2R1bGUYASABKAlSBm1vZHVsZRI0CgZzdGF0dXMYAi'
    'ABKA4yHC5uZXR3b3JraHViLnYxLlNlcnZpbmdTdGF0dXNSBnN0YXR1cxIWCgZkZXRhaWwYAyAB'
    'KAlSBmRldGFpbA==');

@$core.Deprecated('Use conflictDetectedDescriptor instead')
const ConflictDetected$json = {
  '1': 'ConflictDetected',
  '2': [
    {'1': 'module', '3': 1, '4': 1, '5': 9, '10': 'module'},
    {'1': 'detail', '3': 2, '4': 1, '5': 9, '10': 'detail'},
  ],
};

/// Descriptor for `ConflictDetected`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List conflictDetectedDescriptor = $convert.base64Decode(
    'ChBDb25mbGljdERldGVjdGVkEhYKBm1vZHVsZRgBIAEoCVIGbW9kdWxlEhYKBmRldGFpbBgCIA'
    'EoCVIGZGV0YWls');
