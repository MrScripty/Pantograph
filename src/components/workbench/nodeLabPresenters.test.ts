import test from 'node:test';
import assert from 'node:assert/strict';

import type { NodeDefinition, PortDefinition } from '../../services/workflow/types.ts';
import {
  NODE_LAB_IMPLEMENTATION_TEXT,
  NODE_LAB_UNAVAILABLE_FACILITIES,
  buildNodeLabList,
  formatNodeLabDefault,
  nodeLabAuthoringNotice,
  nodeLabDisabledReason,
  nodeLabSummaryLine,
  projectNodeLabDefinition,
  resolveNodeLabSelection,
  sortNodeLabDefinitions,
  type NodeLabAuthoringControl,
} from './nodeLabPresenters.ts';

function port(id: string, data_type: PortDefinition['data_type'], extra: Partial<PortDefinition> = {}): PortDefinition {
  return { id, label: id, data_type, required: false, multiple: false, ...extra };
}

function definition(node_type: string, extra: Partial<NodeDefinition> = {}): NodeDefinition {
  return {
    node_type,
    category: 'processing',
    label: node_type,
    description: '',
    io_binding_origin: 'integrated',
    inputs: [],
    outputs: [],
    execution_mode: 'reactive',
    ...extra,
  };
}

const llm = definition('llm-inference', {
  label: 'LLM Inference',
  description: '  Generates text with a local model. ',
  inputs: [
    port('prompt', 'prompt', { label: 'Prompt', required: true }),
    port('temperature', 'number', { label: 'Temperature', default_value: 0.7 }),
    port('tools', 'tools', { label: 'Tools', multiple: true, description: ' Tool definitions ' }),
  ],
  outputs: [port('response', 'string', { label: 'Response' }), port('stream', 'stream', { label: '' })],
  execution_mode: 'stream',
});
const textInput = definition('text-input', {
  category: 'input',
  label: 'Text Input',
  io_binding_origin: 'client_session',
  outputs: [port('text', 'string', { label: 'Text' })],
});
const textOutput = definition('text-output', { category: 'output', label: 'Text Output' });
const embedding = definition('embedding', { label: 'Embedding', description: 'Embeds text into vectors' });
const custom = definition('mystery', { category: 'experimental' as NodeDefinition['category'], label: 'Aardvark' });

const all = [textOutput, custom, llm, embedding, textInput];

test('the notice names the three unavailable facilities and Settings', () => {
  assert.deepEqual([...NODE_LAB_UNAVAILABLE_FACILITIES], ['Authoring API', 'Local Agent', 'Runtime Publishing']);
  const notice = nodeLabAuthoringNotice();
  assert.equal(notice.headline, 'Node authoring is not available in this build.');
  for (const facility of NODE_LAB_UNAVAILABLE_FACILITIES) {
    assert.ok(notice.sentence.includes(facility), `notice names ${facility}`);
  }
  assert.ok(notice.sentence.includes('Settings'));
});

test('every authoring control has a non-empty reason naming why it is off', () => {
  const controls: NodeLabAuthoringControl[] = [
    'new_node',
    'save_draft',
    'publish',
    'try',
    'edit_field',
    'add_port',
    'remove_port',
  ];
  for (const control of controls) {
    assert.match(nodeLabDisabledReason(control), /this build (does not include|has no Authoring API)/);
  }
  assert.match(nodeLabDisabledReason('publish'), /Runtime Publishing/);
  assert.match(nodeLabDisabledReason('try'), /Local Agent/);
});

test('definitions sort in palette category order, unknown categories last, then by label', () => {
  const before = all.map((d) => d.node_type);
  assert.deepEqual(
    sortNodeLabDefinitions(all).map((d) => d.node_type),
    ['text-input', 'embedding', 'llm-inference', 'text-output', 'mystery'],
  );
  assert.deepEqual(all.map((d) => d.node_type), before, 'input order is untouched');
});

test('the list has one Built-in group with category and port counts', () => {
  const list = buildNodeLabList(all, '');
  assert.equal(list.total, 5);
  assert.equal(list.matched, 5);
  assert.equal(list.emptyText, null);
  assert.deepEqual(list.groups.map((g) => g.label), ['Built-in']);
  assert.deepEqual(list.groups[0].rows[0], {
    nodeType: 'text-input',
    label: 'Text Input',
    categoryLabel: 'Input',
    context: ', 0 inputs, 1 output',
  });
  assert.equal(list.groups[0].rows[2].context, ', 3 inputs, 2 outputs');
  assert.equal(list.groups[0].rows[4].categoryLabel, 'Experimental');
});

test('search matches label, type id, category and description, ignoring case and spaces', () => {
  assert.deepEqual(buildNodeLabList(all, ' TEXT ').groups[0].rows.map((r) => r.nodeType), [
    'text-input',
    'embedding',
    'llm-inference',
    'text-output',
  ]);
  assert.deepEqual(buildNodeLabList(all, 'llm-inf').groups[0].rows.map((r) => r.nodeType), ['llm-inference']);
  assert.deepEqual(buildNodeLabList(all, 'output').groups[0].rows.map((r) => r.nodeType), ['text-output']);
  const none = buildNodeLabList(all, 'zebra ');
  assert.equal(none.matched, 0);
  assert.deepEqual(none.groups, []);
  assert.equal(none.emptyText, 'No built-in node matches “zebra”.');
});

test('an empty registry yields an empty list sentence and summary, not fabricated rows', () => {
  const list = buildNodeLabList([], '');
  assert.deepEqual(list.groups, []);
  assert.equal(list.emptyText, 'No built-in node definitions are loaded.');
  assert.equal(nodeLabSummaryLine([]), 'No node definitions loaded');
  assert.equal(resolveNodeLabSelection([], 'llm-inference'), null);
});

test('summary counts nodes and categories', () => {
  assert.equal(nodeLabSummaryLine(all), '5 built-in nodes in 4 categories, read-only');
  assert.equal(nodeLabSummaryLine([llm]), '1 built-in node in 1 category, read-only');
});

test('selection keeps a reported type and falls back to the first row when it disappears', () => {
  assert.equal(resolveNodeLabSelection(all, 'llm-inference'), llm);
  assert.equal(resolveNodeLabSelection(all, null), textInput);
  assert.equal(resolveNodeLabSelection(all, 'removed-type'), textInput);
  // A search that hides the selected row does not change the detail.
  assert.equal(buildNodeLabList(all, 'output').matched, 1);
  assert.equal(resolveNodeLabSelection(all, 'llm-inference'), llm);
});

test('a definition projects to read-only form rows without invented fields', () => {
  const view = projectNodeLabDefinition(llm);
  assert.equal(view.nodeType, 'llm-inference');
  assert.equal(view.label, 'LLM Inference');
  assert.equal(view.categoryLabel, 'Processing');
  assert.equal(view.description, 'Generates text with a local model.');
  assert.equal(view.executionModeLabel, 'Stream');
  assert.equal(view.ioBindingLabel, 'Integrated');
  assert.equal(view.implementationText, NODE_LAB_IMPLEMENTATION_TEXT);
  assert.equal(view.implementationText, 'built-in, source not editable');
  assert.deepEqual(view.checks, [{ glyph: 'succeeded', text: 'Built-in node, always valid' }]);
  assert.deepEqual(view.inputs, [
    { id: 'prompt', label: 'Prompt', idNote: null, dataType: 'prompt', defaultText: '', requirement: 'required', description: null },
    { id: 'temperature', label: 'Temperature', idNote: null, dataType: 'number', defaultText: '0.7', requirement: 'optional', description: null },
    { id: 'tools', label: 'Tools', idNote: null, dataType: 'tools', defaultText: '', requirement: 'optional, accepts many', description: 'Tool definitions' },
  ]);
  assert.deepEqual(view.outputs.map((p) => [p.id, p.label, p.dataType]), [
    ['response', 'Response', 'string'],
    ['stream', 'stream', 'stream'],
  ]);
  for (const key of ['version', 'runtime', 'device', 'retention']) {
    assert.ok(!(key in view), `${key} is not projected`);
  }
  assert.equal(projectNodeLabDefinition(textInput).ioBindingLabel, 'Client session');
});

test('the port id is noted only when it reads differently from the label', () => {
  const view = projectNodeLabDefinition(
    definition('ports', {
      inputs: [
        port('system_prompt', 'string', { label: 'System Prompt' }),
        port('kv-cache', 'kv_cache', { label: 'KV Cache' }),
        port('pumas_model_ref', 'json', { label: 'Model' }),
      ],
    }),
  );
  assert.deepEqual(view.inputs.map((p) => p.idNote), [null, null, 'pumas_model_ref']);
});

test('a definition missing port arrays projects as having no ports', () => {
  const partial = { ...embedding, inputs: undefined, outputs: undefined } as unknown as NodeDefinition;
  const view = projectNodeLabDefinition(partial);
  assert.deepEqual(view.inputs, []);
  assert.deepEqual(view.outputs, []);
  assert.equal(buildNodeLabList([partial], '').groups[0].rows[0].context, ', 0 inputs, 0 outputs');
});

test('defaults render as text, with structured values as JSON', () => {
  assert.equal(formatNodeLabDefault(undefined), '');
  assert.equal(formatNodeLabDefault(null), '');
  assert.equal(formatNodeLabDefault('hello'), 'hello');
  assert.equal(formatNodeLabDefault(false), 'false');
  assert.equal(formatNodeLabDefault(8), '8');
  assert.equal(formatNodeLabDefault({ k: [1, 2] }), '{"k":[1,2]}');
});
