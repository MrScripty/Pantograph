import test from 'node:test';
import assert from 'node:assert/strict';
import { buildNodeLabStatusRows, nodeLabTrustMessage } from './nodeLabPresenters.ts';

test('NodeLab distinguishes admitted graph authoring from unsupported code publishing', () => {
  assert.match(nodeLabTrustMessage(), /trusted in-process/);
  assert.match(nodeLabTrustMessage(), /cannot install or publish code/);
  assert.deepEqual(buildNodeLabStatusRows(), [
    { label: 'Graph authoring', value: 'Host-admitted nodes' },
    { label: 'Local Agent', value: 'Unavailable' },
    { label: 'Runtime Publishing', value: 'Unsupported' },
  ]);
});
