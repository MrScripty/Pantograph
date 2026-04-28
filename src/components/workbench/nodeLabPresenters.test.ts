import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildNodeLabStatusRows,
  nodeLabUnavailableMessage,
} from './nodeLabPresenters.ts';

test('node lab presenters expose a disabled state without authoring affordances', () => {
  assert.equal(nodeLabUnavailableMessage(), 'Node authoring is unavailable in this build');
  assert.deepEqual(buildNodeLabStatusRows(), [
    { label: 'Authoring API', value: 'Unavailable' },
    { label: 'Local Agent', value: 'Unavailable' },
    { label: 'Runtime Publishing', value: 'Unavailable' },
  ]);
});
