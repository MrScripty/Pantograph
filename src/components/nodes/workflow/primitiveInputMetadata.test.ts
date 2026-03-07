import test from 'node:test';
import assert from 'node:assert/strict';

import {
  findConnectedTargetPort,
  normalizePortDefaultValue,
  parseBooleanNodeValue,
  parseNumberNodeValue,
} from './primitiveInputMetadata.ts';

test('findConnectedTargetPort resolves the downstream input port for a value editor node', () => {
  const result = findConnectedTargetPort(
    'number-1',
    'value',
    [
      {
        id: 'onnx-1',
        data: {
          definition: {
            node_type: 'onnx-inference',
            category: 'processing',
            label: 'ONNX',
            description: '',
            io_binding_origin: 'integrated',
            inputs: [
              {
                id: 'speed',
                label: 'Speed',
                data_type: 'number',
                required: false,
                multiple: false,
                default_value: 1.0,
              },
            ],
            outputs: [],
            execution_mode: 'reactive',
          },
        },
      },
    ],
    [
      {
        source: 'number-1',
        sourceHandle: 'value',
        target: 'onnx-1',
        targetHandle: 'speed',
      },
    ]
  );

  assert.equal(result?.id, 'speed');
  assert.equal(result?.data_type, 'number');
});

test('normalizePortDefaultValue unwraps option objects to runtime values', () => {
  assert.equal(
    normalizePortDefaultValue({ label: 'Leo', value: 'expr-voice-5-m' }),
    'expr-voice-5-m'
  );
  assert.equal(normalizePortDefaultValue(1.2), 1.2);
});

test('parseNumberNodeValue accepts finite numbers and numeric strings', () => {
  assert.equal(parseNumberNodeValue(1.2), 1.2);
  assert.equal(parseNumberNodeValue('1.25'), 1.25);
  assert.equal(parseNumberNodeValue(''), null);
  assert.equal(parseNumberNodeValue('abc'), null);
});

test('parseBooleanNodeValue accepts booleans and boolean strings', () => {
  assert.equal(parseBooleanNodeValue(true), true);
  assert.equal(parseBooleanNodeValue('false'), false);
  assert.equal(parseBooleanNodeValue('maybe'), null);
});
