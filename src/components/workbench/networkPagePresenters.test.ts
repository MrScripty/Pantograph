import test from 'node:test';
import assert from 'node:assert/strict';

import type {
  NodeStatusProjectionRecord,
  SchedulerTimelineProjectionRecord,
} from '../../services/diagnostics/types.ts';
import type {
  WorkflowLocalNetworkNodeStatus,
  WorkflowPeerNetworkNodeStatus,
} from '../../services/workflow/types.ts';
import {
  buildLocalDeviceRows,
  buildLocalDiskRows,
  buildLocalIdentityRows,
  buildLocalInterfaceRows,
  buildLocalNodeEntry,
  buildLocalNodeHeader,
  buildLocalStatCells,
  buildNetworkDimensionLine,
  buildNetworkNodeGroups,
  buildPeerDetail,
  buildPeerNodeEntry,
  buildSelectedRunEventLines,
  buildSelectedRunExecutionRows,
  buildSelectedRunOnLocalNode,
  buildSelectedRunResourceRows,
  describeLocalStatusFailure,
  describeRunProjectionFailure,
  describeSelectedRunForPeer,
  findSelectedRunPlacement,
  formatNetworkBytes,
  formatNetworkClock,
  formatNetworkProjectionFreshness,
  formatSelectedRunLocalState,
  formatSelectedRunModelCacheState,
  formatSelectedRunNodeStatus,
  formatSelectedRunRequirementList,
  formatSelectedRunResourceCacheStatus,
  formatSelectedRunRuntimePosture,
  formatTransportState,
  glyphStateForTransport,
  groupDegradationWarnings,
  logLevelForTimelineEvent,
  resolveSelectedNodeId,
} from './networkPagePresenters.ts';

const GIB = 1_073_741_824;

function createNode(overrides: Partial<WorkflowLocalNetworkNodeStatus> = {}): WorkflowLocalNetworkNodeStatus {
  return {
    node_id: 'local',
    display_name: 'orangecream',
    captured_at_ms: new Date(2026, 10, 3, 20, 5, 51).getTime(),
    transport_state: 'local_only',
    system: {
      hostname: 'orangecream',
      os_name: 'Linux',
      os_version: '7.0',
      kernel_version: '7.0.0-31-generic',
      cpu: {
        logical_core_count: 16,
        average_usage_percent: 12.4,
      },
      memory: {
        total_bytes: 64 * GIB,
        used_bytes: 16 * GIB,
        available_bytes: 48 * GIB,
      },
      disks: [
        {
          name: 'nvme0n1p2',
          mount_point: '/',
          total_bytes: GIB,
          available_bytes: 536_870_912,
        },
      ],
      network_interfaces: [
        {
          name: 'eth0',
          total_received_bytes: 2_048,
          total_transmitted_bytes: 4_096,
        },
      ],
      gpu: {
        available: false,
        reason: 'GPU metrics are not collected by the local system metrics provider',
      },
    },
    scheduler_load: {
      max_sessions: 4,
      active_session_count: 1,
      max_loaded_sessions: 2,
      loaded_session_count: 1,
      active_run_count: 2,
      queued_run_count: 3,
      active_workflow_run_ids: ['run-active'],
      queued_workflow_run_ids: ['run-queued', 'run-queued-unplaced'],
      run_placements: [
        {
          workflow_run_id: 'run-active',
          workflow_execution_session_id: 'session-active',
          workflow_id: 'workflow-a',
          state: 'running',
          runtime_loaded: true,
          model_cache_state: 'cache_hit',
          required_backends: ['llama.cpp'],
          required_models: ['model-a'],
        },
        {
          workflow_run_id: 'run-queued',
          workflow_execution_session_id: 'session-queued',
          workflow_id: 'workflow-a',
          state: 'queued',
          runtime_loaded: false,
          model_cache_state: 'not_required',
          required_backends: [],
          required_models: [],
        },
      ],
    },
    degradation_warnings: [],
    ...overrides,
  };
}

function createPeers(): WorkflowPeerNetworkNodeStatus[] {
  return [
    { node_id: 'node-studio', display_name: 'Studio', transport_state: 'connected', last_seen_at_ms: new Date(2026, 10, 3, 20, 5, 48).getTime() },
    { node_id: 'node-laptop', display_name: 'Laptop', transport_state: 'degraded', last_seen_at_ms: new Date(2026, 10, 3, 19, 48, 10).getTime() },
    { node_id: 'node-render', display_name: 'Render box', transport_state: 'pairing_required', last_seen_at_ms: null },
  ];
}

function createNodeStatus(overrides: Partial<NodeStatusProjectionRecord>): NodeStatusProjectionRecord {
  return {
    workflow_run_id: 'run-active',
    workflow_id: 'workflow-a',
    workflow_version_id: 'wfver-a',
    workflow_semantic_version: '1.0.0',
    node_id: 'node-a',
    node_type: 'image.generate',
    node_version: '1.0.0',
    runtime_id: 'runtime-a',
    runtime_version: '2.0.0',
    task_id: 'image_generation',
    selected_backend_key: 'mock',
    model_id: 'model-a',
    model_version: '3.0.0',
    status: 'running',
    started_at_ms: 1,
    completed_at_ms: null,
    duration_ms: null,
    error: null,
    last_event_seq: 9,
    last_updated_at_ms: 10,
    ...overrides,
  };
}

function createTimelineEvent(overrides: Partial<SchedulerTimelineProjectionRecord>): SchedulerTimelineProjectionRecord {
  return {
    event_seq: 1,
    event_id: 'evt-1',
    event_kind: 'scheduler_queue_placement',
    source_component: 'scheduler',
    occurred_at_ms: new Date(2026, 10, 3, 20, 1, 40, 118).getTime(),
    recorded_at_ms: 0,
    workflow_run_id: 'run-active',
    workflow_id: 'workflow-a',
    summary: 'Run queued',
    payload_json: '{}',
    ...overrides,
  } as SchedulerTimelineProjectionRecord;
}

test('formatters keep compact byte and clock labels', () => {
  assert.equal(formatNetworkBytes(null), 'Unavailable');
  assert.equal(formatNetworkBytes(512), '512 B');
  assert.equal(formatNetworkBytes(2_048), '2.0 KiB');
  assert.equal(formatNetworkBytes(2_097_152), '2.0 MiB');
  assert.equal(formatNetworkBytes(4096 * GIB), '4.0 TiB');
  const at = new Date(2026, 10, 3, 9, 4, 7, 5).getTime();
  assert.equal(formatNetworkClock(at), '09:04:07');
  assert.equal(formatNetworkClock(at, { millis: true }), '09:04:07.005');
  assert.equal(formatNetworkClock(at, { seconds: false }), '09:04');
  assert.equal(formatTransportState('pairing_required'), 'Pairing required');
});

test('transport states map to the glyph vocabulary', () => {
  assert.equal(glyphStateForTransport('connected'), 'succeeded');
  assert.equal(glyphStateForTransport('degraded'), 'cancelled');
  assert.equal(glyphStateForTransport('pairing_required'), 'queued');
  assert.equal(glyphStateForTransport('local_only'), 'running');
  assert.equal(glyphStateForTransport('peer_networking_unavailable'), 'idle');
});

test('node list puts the local node first and keeps peers in projection order', () => {
  const groups = buildNetworkNodeGroups(createNode({ transport_state: 'connected' }), createPeers());

  assert.equal(groups.local.nodeId, 'local');
  assert.equal(groups.local.kind, 'local');
  // The local node reads as local whatever its transport state is.
  assert.equal(groups.local.glyph, 'running');
  assert.equal(groups.local.line1, '2 running, 3 queued');
  assert.equal(groups.local.line2, '16 cores, 16.0 GiB of 64.0 GiB memory');
  assert.equal(groups.local.time, 'local');

  assert.deepEqual(
    groups.peers.map((peer) => [peer.nodeId, peer.glyph, peer.line1, peer.time]),
    [
      ['node-studio', 'succeeded', 'Connected', '20:05'],
      ['node-laptop', 'cancelled', 'Degraded', '19:48'],
      ['node-render', 'queued', 'Pairing required', ''],
    ],
  );
  assert.equal(groups.peers[2].line2, 'Details hidden until paired');
  assert.equal(groups.peers[0].line2, 'Metrics not reported for peers');
  assert.equal(groups.peers[0].timeTitle, 'Last seen 20:05:48');
});

test('selected node falls back to the local node when the peer is gone', () => {
  const groups = buildNetworkNodeGroups(createNode(), createPeers());

  assert.equal(resolveSelectedNodeId(groups, null), 'local');
  assert.equal(resolveSelectedNodeId(groups, 'node-laptop'), 'node-laptop');
  assert.equal(resolveSelectedNodeId(groups, 'node-forgotten'), 'local');
  assert.equal(resolveSelectedNodeId(groups, 'local'), 'local');
});

test('dimension line counts nodes by transport and omits unreported GPU totals', () => {
  assert.equal(
    buildNetworkDimensionLine(createNode(), createPeers(), false),
    '4 nodes, 2 connected, 1 degraded, 1 waiting to be paired.',
  );
  assert.equal(buildNetworkDimensionLine(createNode(), [], false), '1 node, 1 connected.');
  assert.equal(
    buildNetworkDimensionLine(
      createNode({ transport_state: 'peer_networking_unavailable' }),
      [{ node_id: 'p', display_name: 'P', transport_state: 'peer_networking_unavailable' }],
      false,
    ),
    '2 nodes, 1 connected, 1 without peer networking.',
  );
  assert.equal(buildNetworkDimensionLine(null, [], true), 'Reading local status');
  assert.equal(buildNetworkDimensionLine(null, [], false), 'Local status unavailable');
});

test('degradation warnings go to the section they qualify', () => {
  const node = createNode({
    degradation_warnings: [
      'GPU metrics are not collected by the local system metrics provider',
      'CPU usage sample was too short',
      'Memory totals unavailable inside the sandbox',
      'Disk metrics skipped for network mounts',
      'Network interface counters unavailable',
      'Peer discovery is disabled',
      'Metrics captured late',
      'CPU usage sample was too short',
    ],
  });
  const grouped = groupDegradationWarnings(node);

  // The GPU reason already shows in Devices, so the duplicate warning is dropped.
  assert.deepEqual(grouped.devices, []);
  assert.deepEqual(grouped.cpu, ['CPU usage sample was too short']);
  assert.deepEqual(grouped.memory, ['Memory totals unavailable inside the sandbox']);
  assert.deepEqual(grouped.disks, ['Disk metrics skipped for network mounts']);
  assert.deepEqual(grouped.interfaces, ['Network interface counters unavailable']);
  assert.deepEqual(grouped.node, ['Peer discovery is disabled', 'Metrics captured late']);
  assert.deepEqual(groupDegradationWarnings(createNode({ degradation_warnings: ['CUDA probe failed'] })).devices, [
    'CUDA probe failed',
  ]);
});

test('local detail presenters render reported facts and name what is missing', () => {
  const node = createNode();
  const header = buildLocalNodeHeader(node, 0);
  assert.equal(header.subtitle, 'Linux 7.0, captured 20:05:51');
  assert.equal(header.transportLabel, 'Local only');
  assert.deepEqual(header.chips, ['16 cores', 'No GPU reported']);
  assert.deepEqual(buildLocalNodeHeader(createNode({ transport_state: 'connected' }), 3).chips, [
    '16 cores',
    'No GPU reported',
    '3 peers',
  ]);

  const cells = buildLocalStatCells(node);
  assert.deepEqual(
    cells.map((cell) => [cell.id, cell.value, cell.label, cell.meterPercent]),
    [
      ['cpu', '12%', '16 cores', 12.4],
      ['memory', '16.0 GiB', 'of 64.0 GiB memory in use', 25],
      ['scheduler', '2 / 3', 'running / queued here', null],
      ['sessions', '1 of 2', 'runtime sessions loaded, 1 of 4 active', null],
    ],
  );
  const unreported = buildLocalStatCells(
    createNode({
      system: { ...node.system, cpu: { logical_core_count: 1, average_usage_percent: null }, memory: { total_bytes: 0, used_bytes: 0, available_bytes: 0 } },
    }),
  );
  assert.deepEqual([unreported[0].value, unreported[0].label, unreported[0].meterPercent], ['Not reported', '1 core, usage not reported', null]);
  assert.equal(unreported[1].meterPercent, null);

  assert.deepEqual(buildLocalDeviceRows(node), [
    { key: 'GPU', value: 'Not available' },
    { key: 'Reason', value: 'GPU metrics are not collected by the local system metrics provider' },
  ]);
  assert.deepEqual(
    buildLocalDeviceRows(createNode({ system: { ...node.system, gpu: { available: true, reason: null } } })),
    [{ key: 'GPU', value: 'Available' }],
  );
  assert.deepEqual(buildLocalDiskRows(node), [
    { key: 'nvme0n1p2\u{1f}/', mount: '/', name: 'nvme0n1p2', free: '512.0 MiB', total: '1.0 GiB' },
  ]);
  assert.deepEqual(buildLocalInterfaceRows(node), [{ name: 'eth0', received: '2.0 KiB', sent: '4.0 KiB' }]);
  assert.deepEqual(
    buildLocalIdentityRows(createNode({ system: { ...node.system, hostname: null, kernel_version: null } })).map((row) => row.value),
    ['local', 'Not reported', 'Linux 7.0', 'Not reported'],
  );
});

test('peer detail reports only transport and last-seen facts', () => {
  const [studio, , render] = createPeers();
  const detail = buildPeerDetail(studio);
  assert.equal(detail.header.glyph, 'succeeded');
  assert.equal(detail.header.subtitle, 'Last seen 20:05:48');
  assert.match(detail.note, /reports only the transport state and last-seen time/);
  assert.deepEqual(detail.identityRows.map((row) => row.value), ['node-studio', 'Connected', '20:05:48']);

  const pairing = buildPeerDetail(render);
  assert.deepEqual(pairing.header.chips, []);
  assert.equal(pairing.header.glyph, 'queued');
  assert.equal(pairing.header.subtitle, 'Not seen yet');
  assert.match(pairing.note, /not trusted yet/);
  assert.equal(buildPeerNodeEntry(render).timeTitle, undefined);
  assert.equal(buildLocalNodeEntry(createNode()).timeTitle, 'This node');
});

test('selected run placement on the local node uses the placement record', () => {
  const node = createNode();

  const placed = buildSelectedRunOnLocalNode(node, { workflow_run_id: 'run-active', workflow_id: 'ingest-docs-rag' });
  assert.equal(placed.heading, 'ingest-docs-rag on this node');
  assert.equal(placed.state, 'Running locally');
  assert.equal(placed.placed, true);
  assert.equal(placed.note, null);
  assert.deepEqual(placed.rows, [
    { key: 'Runtime session', value: 'Runtime session loaded, model cache hit' },
    { key: 'Session id', value: 'session-active', mono: true },
    { key: 'Backends', value: 'llama.cpp' },
    { key: 'Models', value: 'model-a' },
  ]);

  const fromPlacement = buildSelectedRunOnLocalNode(node, { workflow_run_id: 'run-queued' });
  assert.equal(fromPlacement.heading, 'workflow-a on this node');
  assert.equal(fromPlacement.state, 'Queued locally');

  const finished = buildSelectedRunOnLocalNode(node, { workflow_run_id: 'run-done', workflow_id: null });
  assert.equal(finished.heading, 'Selected run on this node');
  assert.equal(finished.state, 'Not running or queued here');
  assert.equal(finished.placed, false);
  assert.deepEqual(finished.rows, []);
  assert.match(finished.note ?? '', /lists only runs that are running or queued/);

  const listedWithoutPlacement = buildSelectedRunOnLocalNode(node, { workflow_run_id: 'run-queued-unplaced' });
  assert.equal(listedWithoutPlacement.state, 'Queued locally');
  assert.match(listedWithoutPlacement.note ?? '', /placement record is not in the local status yet/);
});

test('selected run with a peer open never claims peer placement', () => {
  const node = createNode();
  assert.deepEqual(describeSelectedRunForPeer(node, { workflow_run_id: 'run-active', workflow_id: 'wf' }, 'Studio'), {
    heading: 'wf',
    sentence: 'wf is running on orangecream, not on Studio.',
  });
  assert.match(
    describeSelectedRunForPeer(node, { workflow_run_id: 'run-done', workflow_id: 'wf' }, 'Studio').sentence,
    /does not report run placement on peers/,
  );
});

test('placement helpers keep their wording', () => {
  const node = createNode();
  const placement = findSelectedRunPlacement(node, 'run-active');
  assert.equal(findSelectedRunPlacement(node, null), null);
  assert.equal(formatSelectedRunLocalState(node, null), 'No selected run');
  assert.equal(formatSelectedRunRuntimePosture(placement), 'Runtime session loaded');
  assert.equal(formatSelectedRunRuntimePosture(null), 'Unavailable');
  assert.equal(formatSelectedRunModelCacheState('unknown'), 'Cache state unknown');
  assert.equal(formatSelectedRunRequirementList([], 'No models'), 'No models');
});

test('execution rows carry the node status glyph', () => {
  const rows = buildSelectedRunExecutionRows([
    createNodeStatus({ node_id: 'node-a', status: 'running' }),
    createNodeStatus({
      node_id: 'node-b',
      status: 'waiting',
      runtime_id: null,
      runtime_version: null,
      model_id: '',
      model_version: null,
    }),
    createNodeStatus({ node_id: 'node-c', status: 'completed' }),
  ]);

  assert.deepEqual(rows, [
    { nodeId: 'node-a', status: 'Running', glyph: 'running', runtime: 'runtime-a@2.0.0', model: 'model-a@3.0.0' },
    { nodeId: 'node-b', status: 'Waiting', glyph: 'waiting', runtime: 'Runtime unavailable', model: 'Model unavailable' },
    { nodeId: 'node-c', status: 'Completed', glyph: 'succeeded', runtime: 'runtime-a@2.0.0', model: 'model-a@3.0.0' },
  ]);
  assert.equal(formatSelectedRunNodeStatus('cancelled'), 'Cancelled');
});

test('resource rows describe library usage in sentence case', () => {
  const rows = buildSelectedRunResourceRows([
    {
      asset_id: 'model:llama',
      total_access_count: 4,
      run_access_count: 2,
      total_network_bytes: 2_048,
      last_accessed_at_ms: 1,
      last_operation: 'read',
      last_cache_status: 'cache_hit',
      last_event_seq: 10,
      last_updated_at_ms: 11,
    },
  ]);

  assert.deepEqual(rows, [
    { assetId: 'model:llama', category: 'Model', cacheStatus: 'Cache hit', networkBytes: '2.0 KiB', accessCount: '2' },
  ]);
  assert.equal(formatSelectedRunResourceCacheStatus(null), 'Not reported');
  assert.equal(formatSelectedRunResourceCacheStatus('load_requested'), 'Load requested');
});

test('timeline events become log lines with a level from the error severity', () => {
  const lines = buildSelectedRunEventLines([
    createTimelineEvent({}),
    createTimelineEvent({
      event_id: 'evt-2',
      event_kind: 'diagnostic_error_occurred',
      source_component: 'runtime',
      summary: 'Model load failed',
      detail: 'out of memory',
      error_severity: 'fatal',
    }),
  ]);

  assert.deepEqual(lines, [
    { id: 'evt-1', time: '20:01:40.118', level: 'info', kind: 'scheduler_queue_placement', message: 'Run queued', source: 'scheduler' },
    {
      id: 'evt-2',
      time: '20:01:40.118',
      level: 'error',
      kind: 'diagnostic_error_occurred',
      message: 'Model load failed. out of memory',
      source: 'runtime',
    },
  ]);
  assert.equal(logLevelForTimelineEvent('warning'), 'warn');
  assert.equal(logLevelForTimelineEvent('error'), 'error');
  assert.equal(logLevelForTimelineEvent(null), 'info');
});

test('failures read as sentences and keep the last capture visible', () => {
  assert.equal(
    describeLocalStatusFailure('Transport error: host unreachable', null),
    'Pantograph could not read the local status. Transport error: host unreachable.',
  );
  assert.equal(
    describeLocalStatusFailure('Internal error: probe failed.', new Date(2026, 10, 3, 20, 5, 51).getTime()),
    'The last refresh failed, so this is the status captured at 20:05:51. Internal error: probe failed.',
  );
  assert.equal(
    describeRunProjectionFailure('node status for this run', 'Scheduler busy'),
    'Could not read node status for this run. Scheduler busy.',
  );
});

test('formatNetworkProjectionFreshness keeps the projection cursor visible', () => {
  assert.equal(formatNetworkProjectionFreshness(null), 'Projection unavailable');
  assert.equal(
    formatNetworkProjectionFreshness({
      projection_name: 'node_status',
      projection_version: 5,
      last_applied_event_seq: 42,
      status: 'rebuilding',
      rebuilt_at_ms: null,
      updated_at_ms: 5,
    }),
    'Rebuilding at seq 42',
  );
});
