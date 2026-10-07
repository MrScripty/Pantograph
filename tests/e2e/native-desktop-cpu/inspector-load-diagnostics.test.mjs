import test from 'node:test';
import assert from 'node:assert/strict';
import { installInspectorLoadDiagnostics, readInspectorLoadDiagnostics } from './inspector-load-diagnostics.mjs';

function install() {
  const listeners = new Map();
  const invoke = () => Promise.resolve();
  const native = {};
  Object.defineProperty(native, 'invoke', { value: invoke });
  globalThis.window = { __TAURI_INTERNALS__: native,
    addEventListener: (name, listener) => listeners.set(name, listener) };
  installInspectorLoadDiagnostics();
  return { native, invoke, failures: window.__nativeInspectorLoadFailures, listeners };
}

test('JavaScript failure observation respects the real read-only Tauri transport', () => {
  const { native, invoke, failures, listeners } = install();
  assert.equal(native.invoke, invoke);
  assert.equal(Object.getOwnPropertyDescriptor(native, 'invoke').writable, false);
  listeners.get('error')({ message: 'statuses is not iterable', filename: 'app.js', lineno: 1, colno: 2 });
  listeners.get('unhandledrejection')({ reason: new Error('render failure') });
  assert.deepEqual(failures.map(record => record.phase), ['javascript_error', 'unhandled_rejection']);
  assert.equal(failures[0].error, 'statuses is not iterable');
});

test('load diagnostics keep a pending selected run distinct from graph and artifact display acceptance', () => {
  install();
  const header = { textContent: ' run-1 ' };
  globalThis.document = { querySelector: () => ({ textContent: 'Loading run snapshot Projection unavailable',
    querySelector: selector => selector === 'h1 + div' ? header : null,
    querySelectorAll: () => [] }) };
  const result = readInspectorLoadDiagnostics();
  assert.equal(result.selectedRunHeader, 'run-1');
  assert.equal(result.loadingSnapshot, true);
  assert.equal(result.projectionUnavailable, true);
  assert.equal(result.capturedGraphPresent, false);
  assert.equal(result.artifactCardCount, 0);
  assert.equal(result.previewCount, 0);
});
