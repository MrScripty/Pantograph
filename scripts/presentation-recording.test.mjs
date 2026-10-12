import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

const source = await fs.readFile(new URL('../docs/presentation/app.js', import.meta.url), 'utf8');
function element() {
  return {
    textContent: '', children: [], listeners: {},
    set innerHTML(_) { assert.fail('Recording data must never enter an HTML sink'); },
    setAttribute(name, value) { this[name] = value; },
    removeAttribute(name) { delete this[name]; },
    replaceChildren() { this.children = []; },
    addEventListener(name, handler) { this.listeners[name] = handler; },
    append(child) { this.children.push(child); },
    get firstElementChild() { return this.children[0]; },
    click() { this.onclick?.(); },
  };
}
test('only exact recordings can become requests/downloads and hostile event text stays text', async () => {
  const nodes = Object.fromEntries(['recording', 'download', 'event-list', 'event-detail', 'trace-summary'].map(id => [id, element()]));
  nodes.recording.value = 'success';
  const hostile = '<img src=x onerror="globalThis.injected=true">';
  const requests = [];
  const context = vm.createContext({
    document: { getElementById: id => nodes[id], querySelectorAll: () => [], createElement: element },
    fetch: async path => {
      requests.push(path);
      return { ok: true, json: async () => ({ execution: { ok: true, result: { outputs: [{ value: hostile }], workflow_run_id: hostile } }, diagnostics: [{ event_seq: 1, event_kind: hostile, node_id: hostile, payload_json: JSON.stringify({ hostile }) }] }) };
    },
  });
  vm.runInContext(source, context);
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(nodes.download.href, 'images/scheduler-demo-success.json');
  assert.ok(nodes['event-list'].children[0].textContent.includes(hostile));
  assert.ok(nodes['event-detail'].textContent.includes('onerror'));
  assert.equal(context.injected, undefined);
  for (const name of ['../private', 'javascript:alert(1)', 'success.json#', '__proto__', 'constructor', 'rejection\r\nX:bad']) {
    nodes.recording.value = name;
    await nodes.recording.listeners.change();
    assert.equal(nodes.download.href, undefined, name);
    assert.match(nodes['trace-summary'].textContent, /unknown recording/);
  }
  assert.deepEqual(requests, ['images/scheduler-demo-success.json']);
  nodes.recording.value = 'rejection';
  await nodes.recording.listeners.change();
  assert.equal(nodes.download.href, 'images/scheduler-demo-rejection.json');
  assert.deepEqual(requests, ['images/scheduler-demo-success.json', 'images/scheduler-demo-rejection.json']);
});
