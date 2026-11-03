import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { applyWorkflowToolbarEvent } from '../../workflowToolbarEvents.ts';
import {
  decodeImageArtifactBody,
  decodeImageArtifactReference,
  ImageOutputError,
  readImageArtifact,
} from './imageOutputArtifact.ts';

function expectImageError(code: ImageOutputError['code'], message: string) {
  return (error: unknown) => {
    assert.ok(error instanceof ImageOutputError);
    assert.equal(error.code, code);
    assert.equal(error.message, message);
    return true;
  };
}

// Independent producer fixture; its media type is a host identifier, not MIME.
const host = JSON.parse(readFileSync(new URL(
  '../../../../crates/pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_completed_outputs.json',
  import.meta.url,
), 'utf8'));
const hostReference = host.outputs.find((output: { port_id: string }) => output.port_id === 'image').value.value;

test('real Rust typed media output reaches image node without string coercion or MIME guessing', () => {
  const updates = new Map<string, Record<string, unknown>>();
  applyWorkflowToolbarEvent({
    event: { type: 'NodeCompleted', data: { node_id: 'inference', outputs: { image: hostReference } } },
    activeWorkflowRunId: null,
    waitingForInput: false,
    edges: [{ id: 'image-edge', source: 'inference', sourceHandle: 'image', target: 'output', targetHandle: 'image' }],
    workflow: {
      setNodeExecutionState() {},
      updateNodeRuntimeData(id, data) { updates.set(id, data); },
      appendStreamContent() {},
      setStreamContent() {},
    },
  });
  assert.deepEqual(decodeImageArtifactReference(updates.get('output')?.image), hostReference);
  assert.equal(hostReference.media_type, 'image_png');
});

test('media reference decoder follows Rust identity bounds and preserves original strings', () => {
  for (const artifact_id of ['id', 'a'.repeat(256), 'é'.repeat(128), '\n id \n', '\u0085id\u0085', '\ufeff']) {
    assert.equal(decodeImageArtifactReference({ artifact_id }).artifact_id, artifact_id);
    assert.deepEqual(decodeImageArtifactReference({ artifact_id, media_type: null }), { artifact_id, media_type: null });
  }
  for (const value of [null, {}, { artifact_id: '' }, { artifact_id: '\u0085' },
    { artifact_id: 'a'.repeat(257) }, { artifact_id: 'é'.repeat(129) },
    { artifact_id: 'a\u0000b' }, { artifact_id: 'a\u0085b' },
    { artifact_id: '\ud800' }, { artifact_id: '\udfff' },
    { artifact_id: 'id', media_type: 1 }, { artifact_id: 'id', media_type: '' },
    { artifact_id: 'id', path: '/tmp/image.png' }]) {
    assert.throws(() => decodeImageArtifactReference(value),
      expectImageError('invalid', 'Invalid image artifact reference.'));
  }
  for (const value of ['aW1hZ2U=', [{ artifact_id: 'id' }]]) {
    assert.throws(() => decodeImageArtifactReference(value),
      expectImageError('unsupported', 'Image output requires one typed artifact reference.'));
  }
});

const reference = { artifact_id: 'image-result', media_type: 'image_png' };
function descriptor() {
  return { artifact: {
    artifact_id: reference.artifact_id,
    payload_kind: 'image', lifecycle_state: 'retained', retention_state: 'retained',
    byte_length: 4, access_modes: ['read'], format: { media_type: 'image/jpeg' },
  } };
}
function read() {
  return {
    response: { artifact_id: reference.artifact_id, media_type: 'image/jpeg',
      body_transport: 'binary_body', complete: true, byte_length: 4 },
    body: [0xff, 0xd8, 0xff, 0xd9],
  };
}

test('complete retained artifact read preserves exact bytes and authoritative MIME for download', async () => {
  const requests: unknown[] = [];
  const body = await readImageArtifact(reference, {
    async artifactDescriptor(request) { requests.push(request); return descriptor(); },
    async readArtifactBody(request) { requests.push(request); return read(); },
  });
  assert.deepEqual(requests, [{ artifact_id: 'image-result' }, { artifact_id: 'image-result' }]);
  assert.equal(body.mediaType, 'image/jpeg');
  assert.equal(body.blob.type, 'image/jpeg');
  assert.equal(body.downloadName, 'output.jpg');
  assert.deepEqual([...new Uint8Array(await body.blob.arrayBuffer())], read().body);
  // Byte validation deliberately does not claim these short bytes decode to an
  // actual image; raster decoding is exercised in the browser fixture.
});

test('unreadable or nonimage descriptors prevent body access', async () => {
  const cases = [
    [null, 'unavailable', 'Image artifact is unavailable.'],
    [{ artifact: { ...descriptor().artifact, artifact_id: 'other' } }, 'invalid', 'Image artifact descriptor does not match the result.'],
    [{ artifact: { ...descriptor().artifact, payload_kind: 'text' } }, 'unsupported', 'Artifact is not an image.'],
    [{ artifact: { ...descriptor().artifact, retention_state: 'deleted' } }, 'unavailable', 'Image artifact is not retained.'],
    [{ artifact: { ...descriptor().artifact, access_modes: [] } }, 'unavailable', 'Image artifact is not readable.'],
    [{ artifact: { ...descriptor().artifact, access_modes: ['invented'] } }, 'invalid', 'Invalid image artifact access modes.'],
    [{ artifact: { ...descriptor().artifact, format: { media_type: 'image/svg+xml' } } }, 'unsupported', 'Unsupported image preview format.'],
  ] as const;
  for (const [response, code, message] of cases) {
    let bodyReads = 0;
    await assert.rejects(readImageArtifact(reference, {
      async artifactDescriptor() { return response; },
      async readArtifactBody() { bodyReads++; return read(); },
    }), expectImageError(code, message));
    assert.equal(bodyReads, 0);
  }
});

test('complete image body validation rejects each corrupt transport field before byte coercion', () => {
  const valid = read();
  const checks = [
    [{ ...valid, response: { ...valid.response, artifact_id: 'other' } }, 'invalid', 'Image artifact read does not match the result.'],
    [{ ...valid, response: { ...valid.response, body_transport: 'redirect_url' } }, 'unsupported', 'Image artifact binary body is unavailable.'],
    [{ ...valid, response: { ...valid.response, complete: false } }, 'unavailable', 'Image artifact read is incomplete.'],
    [{ ...valid, response: { ...valid.response, media_type: 'text/plain' } }, 'unsupported', 'Unsupported image preview format.'],
    [{ ...valid, response: { ...valid.response, media_type: 'image/png' } }, 'invalid', 'Image artifact media type does not match its descriptor.'],
    [{ ...valid, response: { ...valid.response, byte_length: 3 } }, 'invalid', 'Image artifact byte length does not match its body.'],
  ] as const;
  for (const [value, code, message] of checks) {
    assert.throws(() => decodeImageArtifactBody(value, reference.artifact_id, { mediaType: 'image/jpeg', byteLength: 4 }),
      expectImageError(code, message));
  }
  for (const body of [[-1, 0, 0, 0], [256, 0, 0, 0], [0.5, 0, 0, 0], ['1', 0, 0, 0], new Array(4)]) {
    assert.throws(() => decodeImageArtifactBody({ ...valid, body }, reference.artifact_id, { mediaType: 'image/jpeg', byteLength: 4 }),
      expectImageError('invalid', 'Invalid image artifact bytes.'));
  }
});

test('artifact command errors preserve their original failure instead of presenting empty success', async () => {
  const error = new Error('retained artifact read failed');
  await assert.rejects(readImageArtifact(reference, {
    async artifactDescriptor() { return descriptor(); },
    async readArtifactBody() { throw error; },
  }), candidate => candidate === error);
});
