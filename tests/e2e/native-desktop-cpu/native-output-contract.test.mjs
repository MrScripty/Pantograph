import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { defaultVectorArtifact, completedCpuAttempt } from './native-output-contract.mjs';

// Prior native evidence is a scoped regression input, not qualification of this main-based tree.
const fixture = JSON.parse(readFileSync(new URL('./fixtures/native-output-contract.json', import.meta.url), 'utf8'));
const actual = fixture.observation;
const runId = actual.inspection.run.workflow_run_id;

test('actual completed native default output is the vector sink, without internal inference exports', () => {
  assert.equal(actual.inspection.run.status, 'completed');
  const row = defaultVectorArtifact(actual.artifactQuery.artifacts, runId);
  assert.ok(row);
  assert.equal(row.size_bytes, 160);
  assert.equal(row.artifact_role, 'workflow_output');
  assert.ok(!actual.artifactQuery.artifacts.some((item) => item.producer_node_id === 'infer'));
});

test('another run, another port, another role or a deleted artifact cannot satisfy native output', () => {
  const row = defaultVectorArtifact(actual.artifactQuery.artifacts, runId);
  for (const patch of [{ workflow_run_id: 'run_other' }, { producer_node_id: 'infer' },
    { producer_port_id: 'embedding' }, { artifact_role: 'node_output' },
    { retention_state: 'deleted' }, { lifecycle_state: 'deleted' }]) {
    assert.equal(defaultVectorArtifact([{ ...row, ...patch }], runId), undefined);
  }
  assert.equal(defaultVectorArtifact([], runId), undefined);
});

const cpu = { workflow_run_id: runId, event_kind: 'scheduler_task_attempt_lifecycle_changed',
  scheduler_task_id: 'infer', scheduler_attempt_execution_class: 'runtime',
  scheduler_attempt_transition: 'completed', scheduler_attempt_runtime_id: 'candle',
  scheduler_attempt_runtime_variant_id: 'candle.cpu', scheduler_attempt_device_id: 'cpu' };

test('only a scoped completed Candle CPU runtime attempt proves the selection', () => {
  assert.equal(completedCpuAttempt([cpu], runId), cpu);
  for (const patch of [{ workflow_run_id: 'run_other' }, { scheduler_task_id: 'vectors' },
    { scheduler_attempt_transition: 'failed' }, { scheduler_attempt_execution_class: 'non_runtime_node_engine' },
    { scheduler_attempt_runtime_id: 'other' }, { scheduler_attempt_runtime_variant_id: 'candle.cuda' },
    { scheduler_attempt_device_id: 'cuda:0' }]) {
    assert.equal(completedCpuAttempt([{ ...cpu, ...patch }], runId), undefined);
  }
});
