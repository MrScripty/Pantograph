import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compile, parse } from 'svelte/compiler';
import vm from 'node:vm';
import { render } from 'svelte/server';
import type { Component } from 'svelte';
import type { NodeGroup } from '../services/workflow/groupTypes.ts';
import type { WorkflowGraphCurrentValidationSummaryResponse } from '../services/workflow/types.ts';
import { workflowValidationRefreshKey } from './workflowToolbarEvents.ts';
import { groupDisplay } from './nodes/workflow/groupDisplayDiagnostics.ts';
import { WorkflowValidationReadFence, currentWorkflowValidation } from './workflowValidationAuthority.ts';

const moduleUrl = (code: string): string => `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`;
async function component(path: string, overrides: Record<string, string> = {}): Promise<Component> {
  const url = new URL(path, import.meta.url);
  const code = compile(readFileSync(url, 'utf8'), { filename: url.pathname, generate: 'server' }).js.code;
  const linked = code.replace(/from (['"])([^'"]+)\1/g, (_match, _quote, specifier: string) => {
    const resolved = overrides[specifier] ?? (specifier.startsWith('.') ? new URL(`${specifier}.ts`, url).href : import.meta.resolve(specifier));
    return `from '${resolved}'`;
  });
  return (await import(moduleUrl(linked))).default;
}

const group: NodeGroup = {
  id: 'group-example', name: 'Extract result',
  nodes: [{ id: 'extract', node_type: 'json-filter', position: { x: 0, y: 0 }, data: { path: 'results[0].text', opaque: [null, 7, '🐾'] } }],
  edges: [], exposed_inputs: [{ internal_node_id: 'extract', internal_port_id: 'json', group_port_id: 'input', group_port_label: 'JSON', data_type: 'json' }],
  exposed_outputs: [{ internal_node_id: 'extract', internal_port_id: 'value', group_port_id: 'output', group_port_label: 'Result', data_type: 'any' }],
  position: { x: 0, y: 0 }, collapsed: true,
};
const data = () => ({ group: structuredClone(group) });

const stores = moduleUrl(`import { writable } from '${import.meta.resolve('svelte/store')}';
  export const currentSessionId=writable('session'); export const isReadOnly=writable(false);
  export const expandedGroupId=writable(null); export const isEditing=writable(true);
  export function tabIntoGroup(){throw new Error('Rendering must not open groups');}`);
const card = await component('./nodes/workflow/NodeGroupNode.svelte', {
  '@xyflow/svelte': moduleUrl(`export const Position={Left:'left',Right:'right'};export function Handle(r,p){r.push('<i data-port="'+p.id+'"></i>');}`),
  './GroupJsonFilterPathEditor.svelte': moduleUrl('export default function Editor(){throw new Error("Collapsed render must not mutate");}'),
  '../../../stores/graphSessionStore': stores,
  '../../../stores/workflowStore': stores,
});
const panel = await component('./WorkflowValidationDiagnostics.svelte');
function cardHtml(value: unknown): string {
  return render(card, { props: { id: group.id, data: value } }).body;
}

// These are controlled presentation fixtures, not producer admission receipts.
function validation(): WorkflowGraphCurrentValidationSummaryResponse {
  return {
    graph_session_id: 'session', requested_graph_revision: 'revision', current_graph_revision: 'revision',
    state: 'current', submit_gate: { allowed: false, reason_code: 'blocking_diagnostics', message: 'Group cannot execute' },
    diagnostics: [{ severity: 'error', code: 'fixture_unsupported_group',
      message: "Group 'group-example' contains unsupported processing type 'llm-inference'.",
      hint: 'Keep the workflow saved and use a supported processing group.', port_id: 'input' }],
  };
}
const panelHtml = (value: WorkflowGraphCurrentValidationSummaryResponse | null, graphSessionId = 'session', graphRevision = 'revision') =>
  render(panel, { props: { validation: value, graphSessionId, graphRevision } }).body;

test('valid grouped card retains exact child data, ports and path authoring', () => {
  const saved = data(); const before = JSON.stringify(saved);
  const display = groupDisplay(saved);
  assert.equal(display.nodes[0], saved.group.nodes[0]);
  assert.equal(display.diagnostics.length, 0);
  assert.deepEqual(display.inputs.map(p => p.id), ['input']);
  const html = cardHtml(saved);
  assert.match(html, /Extract result/); assert.match(html, /JSON Filter paths/);
  assert.match(html, /data-port="input"/); assert.match(html, /data-port="output"/);
  assert.doesNotMatch(html, /Group editor unavailable/);
  assert.equal(JSON.stringify(saved), before);
});

test('malformed grouped examples render actionable fields without mutation or editor controls', () => {
  const malformed: Array<[unknown, string]> = [
    [null, 'data.group'], [{ group: [] }, 'data.group'],
    [{ group: { ...group, nodes: {} } }, 'data.group.nodes'],
    [{ group: { ...group, nodes: [null] } }, 'data.group.nodes[0]'],
    [{ group: { ...group, nodes: [group.nodes[0], group.nodes[0]] } }, 'data.group.nodes[1].id'],
    [{ group: { ...group, exposed_inputs: {} } }, 'data.group.exposed_inputs'],
    [{ group: { ...group, exposed_outputs: [null] } }, 'data.group.exposed_outputs[0]'],
    [{ group: { ...group, exposed_inputs: [group.exposed_inputs[0], { ...group.exposed_inputs[0], internal_node_id: 'other' }] } }, 'data.group.exposed_inputs[1].group_port_id'],
    [{ group: { ...group, exposed_outputs: [{ ...group.exposed_outputs[0], group_port_label: {} }] } }, 'data.group.exposed_outputs[0]'],
  ];
  for (const [saved, field] of malformed) {
    const before = JSON.stringify(saved); const display = groupDisplay(saved);
    assert.ok(display.diagnostics.some(issue => issue.field === field), field);
    const html = cardHtml(saved);
    assert.match(html, /Group editor unavailable/); assert.match(html, /invalid_group_display_data/);
    assert.ok(html.includes(field)); assert.match(html, /Restore a valid saved group/);
    assert.match(html, /disabled/); assert.doesNotMatch(html, /data-port=|JSON Filter paths/);
    assert.equal(JSON.stringify(saved), before);
  }
});

test('card display does not invent execution policy for native, nested or malformed node data', () => {
  for (const kind of ['llm-inference', 'node-group', 'future-producer-type']) {
    const saved = data(); saved.group.nodes[0].node_type = kind;
    assert.equal(groupDisplay(saved).diagnostics.length, 0);
    assert.doesNotMatch(cardHtml(saved), /Group editor unavailable|supported processing/);
  }
  const saved = data(); saved.group.nodes[0].data = null as unknown as Record<string, unknown>;
  assert.equal(groupDisplay(saved).nodes[0].data, null);
  assert.match(cardHtml(saved), /JSON Filter paths/);
});

test('typed backend diagnostics expose severity, code, port and original remediation before submission', () => {
  const projection = validation(); const before = JSON.stringify(projection);
  const html = panelHtml(projection);
  assert.match(html, /Workflow validation: current/); assert.match(html, /fixture_unsupported_group/);
  assert.match(html, /llm-inference/); assert.match(html, /Port: input/);
  assert.match(html, /How to fix: Keep the workflow saved/);
  assert.equal(JSON.stringify(projection), before);
});

test('diagnostics stay scoped to the exact session and requested/current revision', () => {
  assert.doesNotMatch(panelHtml(null), /workflow-validation-diagnostic/);
  assert.doesNotMatch(panelHtml(validation(), 'other-session'), /workflow-validation-diagnostic/);
  assert.doesNotMatch(panelHtml(validation(), 'session', 'new-revision'), /workflow-validation-diagnostic/);
  assert.doesNotMatch(panelHtml({ ...validation(), current_graph_revision: 'other' }), /workflow-validation-diagnostic/);
  assert.doesNotMatch(panelHtml({ ...validation(), requested_graph_revision: 'other' }), /workflow-validation-diagnostic/);
});

test('multiple diagnostics, missing hints, invalid state and HTML payloads remain readable and escaped', () => {
  const projection = validation(); projection.state = 'invalid';
  projection.diagnostics!.push({ severity: 'warning', code: '<script>alert(1)</script>', message: '<img src=x onerror=alert(1)>', hint: null });
  const html = panelHtml(projection);
  assert.equal((html.match(/data-testid="workflow-validation-diagnostic"/g) ?? []).length, 2);
  assert.match(html, /Workflow validation: invalid/);
  assert.doesNotMatch(html, /<script>|<img/); assert.match(html, /&lt;script/);
  assert.equal((html.match(/How to fix:/g) ?? []).length, 1);
});


test('public producer fan-out mappings render one handle without rewriting saved mappings', () => {
  const saved = data(); saved.group.exposed_outputs.push(structuredClone(saved.group.exposed_outputs[0]));
  const before = JSON.stringify(saved);
  const display = groupDisplay(saved);
  assert.equal(display.diagnostics.length, 0);
  assert.equal(display.outputs.length, 1);
  const html = cardHtml(saved);
  assert.equal((html.match(/data-port="output"/g) ?? []).length, 1);
  assert.doesNotMatch(html, /Group editor unavailable/);
  assert.equal(saved.group.exposed_outputs.length, 2);
  assert.equal(JSON.stringify(saved), before);
});


function findSyntax(value: unknown, predicate: (node: Record<string, unknown>) => boolean): Record<string, unknown> | undefined {
  if (!value || typeof value !== 'object') return undefined;
  const node = value as Record<string, unknown>;
  if (predicate(node)) return node;
  for (const child of Object.values(node)) {
    const children = Array.isArray(child) ? child : [child];
    for (const item of children) {
      const found = findSyntax(item, predicate);
      if (found) return found;
    }
  }
  return undefined;
}

test('actual Open handler refuses malformed cards and keeps valid group identity', () => {
  const source = readFileSync(new URL('./nodes/workflow/NodeGroupNode.svelte', import.meta.url), 'utf8');
  const syntax = findSyntax(parse(source, { modern: true }), node =>
    node.type === 'FunctionDeclaration' && (node.id as { name?: string })?.name === 'handleOpenGroup')!;
  const handler = source.slice(syntax.start as number, syntax.end as number);
  const calls: string[] = [];
  for (const saved of [null, data()]) {
    const context = vm.createContext({ id: group.id, display: groupDisplay(saved), tabIntoGroup: (id: string) => calls.push(id) });
    vm.runInContext(`${handler}; handleOpenGroup();`, context);
  }
  assert.deepEqual(calls, [group.id]);
});

test('actual toolbar refresh clears old summary and ignores cancelled or superseded success/failure', async () => {
  const source = readFileSync(new URL('./WorkflowToolbar.svelte', import.meta.url), 'utf8');
  const syntax = findSyntax(parse(source, { modern: true }), node => {
    if (node.type !== 'CallExpression' || (node.callee as { name?: string })?.name !== '$effect') return false;
    return source.slice(node.start as number, node.end as number).includes('.refreshCurrentGraphValidationSummary');
  })!;
  const callback = (syntax.arguments as { start: number; end: number }[])[0];
  for (const change of ['cleanup', 'revision-switch', 'session-switch', 'late-failure']) {
    let resolve!: (value: unknown) => void;
    let reject!: (error: Error) => void;
    const response = new Promise((done, fail) => { resolve = done; reject = fail; });
    const context = vm.createContext({
      $currentSessionId: 'session', $currentGraphType: 'workflow',
      $workflowGraph: { derived_graph: { graph_fingerprint: 'new-revision' } },
      currentValidationSummary: validation(), currentValidationSummaryKey: 'session:revision',
      workflowValidationRefreshKey, validationReadFence: new WorkflowValidationReadFence(),
      acceptedValidationRead: null, currentWorkflowValidation,
      clearNodeRuntimeData: () => {}, INFERENCE_INTERFACE_VALIDATION_RUNTIME_KEYS: [],
      workflowService: { refreshCurrentGraphValidationSummary: () => response },
      workflowValidationProjectionOverlays: () => [], updateNodeRuntimeData: () => {},
    });
    const cancel = vm.runInContext(`(${source.slice(callback.start, callback.end)})()`, context);
    assert.equal(context.currentValidationSummary, null);
    assert.equal(context.currentValidationSummaryKey, 'session:new-revision');
    const newerSummary = validation();
    if (change === 'cleanup') cancel();
    else {
      context.currentValidationSummaryKey = change === 'session-switch' ? 'other:new-revision' : 'session:newer-revision';
      context.currentValidationSummary = newerSummary;
      context.validationReadFence.begin(context.currentValidationSummaryKey);
    }
    if (change === 'late-failure') reject(new Error('Previous request failed'));
    else resolve({ summary: validation(), node_projections: [] });
    await new Promise(done => setImmediate(done));
    assert.equal(context.currentValidationSummary, change === 'cleanup' ? null : newerSummary, change);
  }
});
