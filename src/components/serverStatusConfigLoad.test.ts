import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import { parse } from 'svelte/compiler';

// Execute the actual component's mount and connect handlers with controlled services.
// This exercises ordering across awaited config reads without claiming browser rendering.
const source = readFileSync(new URL('./ServerStatus.svelte', import.meta.url), 'utf8');
const script = parse(source, { modern: true }).instance!.content.body;
const mount = script.find(node => node.type === 'ExpressionStatement'
  && node.expression.type === 'CallExpression'
  && node.expression.callee.type === 'Identifier'
  && node.expression.callee.name === 'onMount');
const connect = script.find(node => node.type === 'FunctionDeclaration'
  && node.id?.name === 'connectExternal');
assert.ok(mount && connect);
function sourceOf(node: unknown): string {
  const { start, end } = node as { start: number; end: number };
  assert.equal(typeof start, 'number');
  assert.equal(typeof end, 'number');
  return source.slice(start, end);
}
const handlers = stripTypeScriptTypes(sourceOf(mount)
  + '\n' + sourceOf(connect)
  + '\nglobalThis.connect = connectExternal;');

function fixture(failLoad = false) {
  let finishLoad!: () => void;
  const pending = new Promise<void>(resolve => { finishLoad = resolve; });
  const saved: unknown[] = [];
  const connections: string[] = [];
  const capacity = [{ id: 'shared-host', capacity_bytes: 100 }];
  let configState = { config: {}, isLoading: false, error: null as string | null };
  let subscriber: ((state: typeof configState) => void) | undefined;
  const context = vm.createContext({
    configReady: false, configState,
    llmState: { status: { mode: 'none', ready: false } },
    externalUrl: 'http://localhost:1234', apiKey: '', isConnecting: false,
    connectionType: 'external', healthState: {}, console,
    onMount: (callback: () => Promise<void>) => { context.mount = callback; },
    ConfigService: {
      getState: () => configState,
      subscribe: (callback: typeof subscriber) => { subscriber = callback; return () => {}; },
      loadConfig: async () => {
        configState.isLoading = true;
        subscriber?.(configState);
        await pending;
        configState = failLoad
          ? { config: {}, isLoading: false, error: 'malformed saved config' }
          : { config: { runtime_resource_domains: capacity }, isLoading: false, error: null };
        subscriber?.(configState);
      },
      refreshServerMode: async () => {},
      saveConfig: async (config: unknown) => { saved.push(config); },
    },
    LLMService: {
      subscribe: () => () => {},
      connectToServer: async (url: string) => { connections.push(url); },
    },
    HealthMonitorService: {
      subscribeState: () => () => {}, subscribeEvents: () => () => {}, start: async () => {},
    },
  });
  vm.runInContext(handlers, context);
  return { context, saved, connections, capacity, finishLoad };
}

test('external connect waits for saved capacity config and preserves it after loading', async () => {
  const f = fixture();
  await f.context.connect();
  assert.deepEqual(f.connections, []);
  const mounting = f.context.mount();
  await f.context.connect();
  assert.deepEqual(f.connections, []);
  assert.deepEqual(f.saved, []);
  f.finishLoad();
  await mounting;
  await f.context.connect();
  assert.deepEqual(f.connections, ['http://localhost:1234']);
  assert.equal(f.saved.length, 1);
  assert.deepEqual((f.saved[0] as { runtime_resource_domains: unknown }).runtime_resource_domains, f.capacity);
});

test('a failed config read cannot connect and overwrite settings with defaults', async () => {
  const f = fixture(true);
  const mounting = f.context.mount();
  f.finishLoad();
  await mounting;
  await f.context.connect();
  assert.deepEqual(f.connections, []);
  assert.deepEqual(f.saved, []);
});
