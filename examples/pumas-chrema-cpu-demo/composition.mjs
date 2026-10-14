import assert from 'node:assert/strict';
import process from 'node:process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { absoluteDirectory, inventory, json, lock, save, sha256, validateNative, verifyInventory } from './contracts.mjs';
import { command, loggedJson } from './process.mjs';

export function verifyPrepared(prepared) {
  const receipt = json(join(prepared, 'prepared.json'));
  assert.equal(receipt.contract_version, 1);
  assert.deepEqual(receipt.compatibility, lock, 'prepared compatibility mismatch');
  verifyInventory(prepared, receipt.files);
  assert.ok(receipt.files[`bin/${lock.native_example}`], 'prepared binary identity missing');
  assert.ok(receipt.files['packages/agent-platform/package.json'], 'prepared Chrema package missing');
  return receipt;
}

export async function inspectNative(prepared, native, log) {
  return command(join(prepared, 'bin', lock.native_example), [native, '--inspect-retained'], { log })
    .then(() => loggedJson(log));
}

export async function runComposition({ prepared, out, timeoutMs = 60_000 }) {
  prepared = absoluteDirectory(prepared);
  out = absoluteDirectory(out);
  if (!Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 60_000) throw new Error('invalid_native_deadline');
  const receipt = verifyPrepared(prepared);
  mkdirSync(out); // Preserve every previous candidate, including failed ones.
  save(join(out, 'dependency-receipt.json'), receipt);
  // Resolve the package's public export; internal Chrema paths are not a contract.
  const packageRoot = join(prepared, 'packages/agent-platform');
  const entry = json(join(packageRoot, 'package.json')).exports['./journal'];
  assert.equal(typeof entry, 'string');
  const { createAgentWorkflowJournalFactory } = await import(pathToFileURL(join(packageRoot, entry)));
  const steps = ['native-inference', 'retained-inspection'];
  const journal = createAgentWorkflowJournalFactory({
    assertCompletion(run, _version, summary) {
      assert.deepEqual(run.assignments.map(assignment => assignment.stepKey), steps);
      assert.ok(run.assignments.every(assignment => assignment.status === 'completed'));
      assert.equal(summary.modelQualification, 'untrained-fixture');
      assert.equal(typeof summary.pantographRunId, 'string');
      assert.equal(summary.vector.length, 8);
    },
    projectMetrics: run => ({ completedComponents: run.assignments.filter(assignment => assignment.status === 'completed').length }),
    observeJournalEvent: async () => {}, observeAttemptOutput: async () => {},
    assignmentMarker: id => `CPU-COMPOSITION:${id}`,
  })();
  const version = await journal.registerVersion({
    id: 'pumas-pantograph-cpu.v1', workflowType: 'cpu-embedding-composition',
    definition: { contractVersion: 1, steps, compatibility: lock }, codeRevision: receipt.pantograph_revision,
    changeReason: 'Compose a Pumas-indexed native workflow with public Chrema.',
    improvementHypothesis: 'Retain actual output and provenance after producer exit.', reviewedBy: 'cpu-demo-host',
  });
  const run = await journal.createRun({ workflowVersionId: version.id,
    owner: { kind: 'system', principalId: 'cpu-demo-host' },
    input: { text: 'hello world', modelId: lock.model_id, modelQualification: 'untrained-fixture' },
  });
  await journal.claimRun({ workerId: 'cpu-demo-host', workflowType: version.workflowType });
  const native = join(out, 'native');
  let assignment;
  let summary;
  let terminalError;
  try {
    for (const step of steps) {
      assignment = await journal.createAssignment({ runId: run.runId, stepKey: step, assignedTo: 'cpu-demo-host', execution: 'current-agent',
        targets: [{ type: 'local-cpu-workflow', id: lock.native_example, expectedVersion: null }],
      });
      await journal.appendEvent({ runId: run.runId, assignmentId: assignment.assignmentId,
        sourceKey: `${step}:started`, eventType: 'cpu-demo.component-started',
        payload: { step, input: run.input, compatibility: lock },
      });
      if (step === 'native-inference') {
        await command(join(prepared, 'bin', lock.native_example), [native], {
          env: { ...process.env, PUMAS_REGISTRY_DB_PATH: join(native, 'registry.db') },
          log: join(out, step), timeoutMs,
        });
        const result = json(join(native, 'result.json'));
        assert.equal(result.qualification.status, 'passed');
        assert.equal(result.shutdown_ok, true);
        await journal.appendEvent({ runId: run.runId, assignmentId: assignment.assignmentId,
          sourceKey: `${step}:completed`, eventType: 'cpu-demo.native-completed',
          payload: { step, pantographRunId: result.qualification.output.response.workflow_run_id,
            resultFile: 'native/result.json', resultSha256: sha256(readFileSync(join(native, 'result.json'))) },
        });
      } else {
        // The producer has exited. This is a new native process, with no model/runtime construction.
        const inspection = await inspectNative(prepared, native, join(out, step));
        save(join(out, 'retained-inspection.json'), inspection);
        summary = { ...validateNative(json(join(native, 'result.json')), json(join(native, 'pumas-model.json')), inspection), modelQualification: 'untrained-fixture' };
        await journal.appendEvent({ runId: run.runId, assignmentId: assignment.assignmentId,
          sourceKey: `${step}:completed`, eventType: 'cpu-demo.retained-inspection-completed',
          payload: { step, ...summary, inspectionFile: 'retained-inspection.json' },
        });
      }
      await journal.finishAssignment({ assignmentId: assignment.assignmentId, status: 'completed' });
      assignment = undefined;
    }
    await journal.transitionRun({ runId: run.runId, status: 'completed', finalSummary: summary });
  } catch (error) {
    terminalError = error;
    if (assignment) await journal.finishAssignment({ assignmentId: assignment.assignmentId, status: 'failed', failure: error.message });
    await journal.transitionRun({ runId: run.runId, status: 'failed', failure: { code: error.message, details: error.details ?? null } });
  }
  const retained = await journal.inspectRun({ runId: run.runId });
  save(join(out, 'chrema-journal.json'), { contract_version: 1, retention: 'exported-memory-journal-inspection-only', version, run: retained });
  const bundle = { contract_version: 1, status: retained.status, chremaRunId: run.runId,
    compatibility: lock, summary: summary ?? null, files: inventory(out) };
  save(join(out, 'bundle.json'), bundle);
  if (terminalError) throw terminalError;
  return { status: 'completed', directory: out, chremaRunId: run.runId, ...summary };
}

export async function inspectBundle({ prepared, out }) {
  prepared = absoluteDirectory(prepared);
  out = absoluteDirectory(out);
  const receipt = verifyPrepared(prepared);
  const bundle = json(join(out, 'bundle.json'));
  assert.equal(bundle.contract_version, 1);
  assert.equal(bundle.status, 'completed', 'composition did not complete');
  assert.deepEqual(bundle.compatibility, lock);
  verifyInventory(out, bundle.files);
  assert.deepEqual(json(join(out, 'dependency-receipt.json')), receipt, 'inspection binary differs from producer');
  const exported = json(join(out, 'chrema-journal.json'));
  assert.equal(exported.contract_version, 1);
  assert.equal(exported.run.runId, bundle.chremaRunId);
  assert.equal(exported.run.status, 'completed');
  assert.deepEqual(exported.run.assignments.map(assignment => [assignment.stepKey, assignment.status]), [
    ['native-inference', 'completed'], ['retained-inspection', 'completed'],
  ]);
  assert.deepEqual(exported.run.finalSummary, bundle.summary);
  assert.deepEqual(exported.version.definition.compatibility, lock);
  // Projection rebuild is an owner-store mutation. Inspect a verified private copy
  // so repeated inspection leaves the sealed evidence byte-for-byte intact.
  const scratch = mkdtempSync(join(tmpdir(), 'pantograph-retained-inspection-'));
  try {
    for (const path of ['result.json', 'diagnostics.db', 'attribution.db', 'artifacts']) {
      cpSync(join(out, 'native', path), join(scratch, path), { recursive: true });
    }
    const inspection = await inspectNative(prepared, scratch, join(scratch, 'inspection'));
    const summary = { ...validateNative(json(join(out, 'native/result.json')), json(join(out, 'native/pumas-model.json')), inspection), modelQualification: 'untrained-fixture' };
    assert.deepEqual(summary, bundle.summary, 'fresh retained-state inspection differs');
    verifyInventory(out, bundle.files);
    return { status: 'verified', directory: out, chremaRunId: bundle.chremaRunId, ...summary };
  } finally {
    rmSync(scratch, { recursive: true });
  }
}
