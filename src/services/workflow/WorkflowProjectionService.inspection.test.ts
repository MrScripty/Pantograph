import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { WorkflowProjectionService } from './WorkflowProjectionService.ts';
import { buildRunGraphCanvasModel, buildRunGraphNodeStatusMap } from '../../components/workbench/runGraphPresenters.ts';
import { formatProjectionFreshness } from '../../components/workbench/ioInspectorPresenters.ts';
import type { WorkflowRunGraphProjection } from './types.ts';

function inspectionWireResponse(runId: string) {
  const graph = { nodes: [{ id: 'vectors', node_type: 'vector-output',
    position: { x: 0, y: 0 }, data: {} }], edges: [] };
  const runGraph: WorkflowRunGraphProjection = {
    workflow_run_id: runId, workflow_id: 'synthetic-cpu',
    workflow_version_id: 'version-1', workflow_presentation_revision_id: 'presentation-1',
    workflow_semantic_version: '1.0.0', workflow_execution_fingerprint: 'fingerprint-1',
    snapshot_created_at_ms: 100, workflow_version_created_at_ms: 100,
    presentation_revision_created_at_ms: 100, graph,
    executable_topology: { schema_version: 1,
      nodes: [{ node_id: 'vectors', node_type: 'vector-output', contract_version: '1', behavior_digest: 'digest-1' }], edges: [] },
    presentation_metadata: { schema_version: 1,
      nodes: [{ node_id: 'vectors', position: { x: 0, y: 0 } }], edges: [] },
    graph_settings: { schema_version: 1,
      nodes: [{ node_id: 'vectors', node_type: 'vector-output', data: {} }] },
  };
  const state = (projection_name: string) => ({ projection_name, projection_version: 6,
    last_applied_event_seq: 12, status: 'current' as const, rebuilt_at_ms: null, updated_at_ms: 100 });
  // Match the Rust Vec::is_empty wire contract, including the native failure's
  // absent node_statuses. No node completion status is invented from run status.
  return { run_graph: runGraph,
    run_projection_state: state('run_detail'), node_projection_state: state('node_status'),
    io_projection_state: state('io_artifact') };
}

test('omitted inspection collections reach graph presenters as empty arrays for the requested run', async () => {
  Object.assign(globalThis, { window: globalThis });
  const calls: unknown[] = [];
  const wire = inspectionWireResponse('run-cpu-1');
  mockIPC((command, args) => {
    calls.push({ command, args });
    return wire;
  });
  try {
    const result = await new WorkflowProjectionService().queryRunInspection({ workflow_run_id: 'run-cpu-1', artifact_limit: 250 });
    assert.deepEqual(calls, [{ command: 'workflow_run_inspection_query',
      args: { request: { workflow_run_id: 'run-cpu-1', artifact_limit: 250 } } }]);
    assert.equal(result.run_graph, wire.run_graph);
    assert.equal(result.io_projection_state, wire.io_projection_state);
    assert.deepEqual(result.node_statuses, []);
    assert.deepEqual(result.io_artifacts, []);
    assert.deepEqual(result.retention_summary, []);
    assert.deepEqual(buildRunGraphNodeStatusMap(result.node_statuses), {});
    const canvas = buildRunGraphCanvasModel(result.run_graph!.graph, {}, buildRunGraphNodeStatusMap(result.node_statuses));
    assert.deepEqual(canvas.nodes.map(node => node.id), ['vectors']);
    assert.equal(canvas.nodes[0].statusClass, 'unknown');
    assert.notEqual(formatProjectionFreshness(result.io_projection_state), 'Projection unavailable');
    assert.equal(Object.hasOwn(wire, 'node_statuses'), false, 'normalization must not mutate the wire response');
  } finally {
    clearMocks();
  }
});

test('inspection preserves populated status and artifact collections, without carrying the previous run into empty responses', async () => {
  Object.assign(globalThis, { window: globalThis });
  const fixture = JSON.parse(readFileSync(new URL('../../../crates/pantograph-workflow-service/tests/fixtures/run_projection_contract.json', import.meta.url), 'utf8'));
  const populated = { ...inspectionWireResponse('run-1'), node_statuses: fixture.run_detail_response.node_statuses,
    io_artifacts: [{ artifact_id: 'payload-vector-1', workflow_run_id: 'run-1' }],
    retention_summary: [{ retention_state: 'retained', artifact_count: 1 }],
    resolved_node_io: [{ node_id: 'vectors', port_id: 'vector' }] };
  mockIPC((_command, args) => {
    const request = (args as { request: { workflow_run_id: string } }).request;
    return request.workflow_run_id === 'run-1' ? populated : inspectionWireResponse('run-2');
  });
  try {
    const service = new WorkflowProjectionService();
    const first = await service.queryRunInspection({ workflow_run_id: 'run-1' });
    assert.equal(first.node_statuses, populated.node_statuses);
    assert.equal(first.io_artifacts, populated.io_artifacts);
    assert.equal(first.retention_summary, populated.retention_summary);
    assert.equal(first.resolved_node_io, populated.resolved_node_io);
    const next = await service.queryRunInspection({ workflow_run_id: 'run-2' });
    assert.equal(next.run_graph?.workflow_run_id, 'run-2');
    assert.deepEqual(next.node_statuses, []);
    assert.deepEqual(next.io_artifacts, []);
    assert.deepEqual(next.retention_summary, []);
  } finally {
    clearMocks();
  }
});
