import test from 'node:test';
import assert from 'node:assert/strict';

import {
  formatDiagnosticsBytes,
  formatDiagnosticsDuration,
  formatDiagnosticsPercent,
  getDiagnosticsStatusClasses,
  getRuntimeInstallStateClasses,
  getRunNodeStatusCounts,
  getSchedulerStateClasses,
} from './presenters.ts';
import type { DiagnosticsRunTrace } from '../../services/diagnostics/types.ts';

function createRunTrace(): DiagnosticsRunTrace {
  return {
    executionId: 'exec-1',
    sessionId: 'session-1',
    workflowId: 'wf-1',
    workflowName: 'Workflow One',
    graphFingerprintAtStart: 'graph-1',
    nodeCountAtStart: 4,
    status: 'running',
    startedAtMs: 1_000,
    endedAtMs: null,
    durationMs: null,
    lastUpdatedAtMs: 1_250,
    error: null,
    waitingForInput: false,
    runtime: {
      runtimeId: 'llama.cpp',
      runtimeInstanceId: 'llama-cpp-1',
      modelTarget: '/models/demo.gguf',
      warmupStartedAtMs: 900,
      warmupCompletedAtMs: 980,
      warmupDurationMs: 80,
      runtimeReused: false,
      lifecycleDecisionReason: 'runtime_ready',
    },
    eventCount: 4,
    streamEventCount: 1,
    lastDirtyTasks: [],
    lastIncrementalTaskIds: [],
    nodes: {
      a: {
        nodeId: 'a',
        nodeType: 'loader',
        status: 'completed',
        startedAtMs: 1_000,
        endedAtMs: 1_020,
        durationMs: 20,
        lastProgress: 1,
        lastMessage: null,
        streamEventCount: 0,
        eventCount: 2,
        error: null,
      },
      b: {
        nodeId: 'b',
        nodeType: 'llm',
        status: 'running',
        startedAtMs: 1_010,
        endedAtMs: null,
        durationMs: null,
        lastProgress: 0.6,
        lastMessage: 'Generating',
        streamEventCount: 1,
        eventCount: 3,
        error: null,
      },
      c: {
        nodeId: 'c',
        nodeType: 'approval',
        status: 'waiting',
        startedAtMs: 1_040,
        endedAtMs: null,
        durationMs: null,
        lastProgress: null,
        lastMessage: 'Awaiting input',
        streamEventCount: 0,
        eventCount: 1,
        error: null,
      },
      d: {
        nodeId: 'd',
        nodeType: 'writer',
        status: 'failed',
        startedAtMs: 1_050,
        endedAtMs: 1_070,
        durationMs: 20,
        lastProgress: null,
        lastMessage: null,
        streamEventCount: 0,
        eventCount: 1,
        error: 'disk full',
      },
    },
    events: [],
  };
}

test('formatDiagnosticsDuration reports milliseconds seconds and in-progress states', () => {
  assert.equal(formatDiagnosticsDuration(null), 'In progress');
  assert.equal(formatDiagnosticsDuration(250), '250 ms');
  assert.equal(formatDiagnosticsDuration(2_500), '2.5 s');
  assert.equal(formatDiagnosticsDuration(12_000), '12 s');
  assert.equal(formatDiagnosticsDuration(125_000), '2m 5s');
});

test('formatDiagnosticsPercent and status classes expose stable labels', () => {
  assert.equal(formatDiagnosticsPercent(null), 'No progress');
  assert.equal(formatDiagnosticsPercent(0.523), '52%');
  assert.match(getDiagnosticsStatusClasses('waiting'), /amber/);
});

test('byte and runtime or scheduler label helpers expose readable labels', () => {
  assert.equal(formatDiagnosticsBytes(512), '512 B');
  assert.equal(formatDiagnosticsBytes(2048), '2.0 KiB');
  assert.match(getSchedulerStateClasses('idle_loaded'), /emerald/);
  assert.match(getRuntimeInstallStateClasses('missing', false), /amber/);
  assert.match(getRuntimeInstallStateClasses('installed', true), /emerald/);
});

test('getRunNodeStatusCounts groups node states for overview summaries', () => {
  const counts = getRunNodeStatusCounts(createRunTrace());
  assert.deepEqual(counts, {
    running: 1,
    waiting: 1,
    completed: 1,
    cancelled: 0,
    failed: 1,
  });
});
