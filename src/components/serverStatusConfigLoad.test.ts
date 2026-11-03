import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import vm from 'node:vm';
import { parse } from 'svelte/compiler';
import type { AppConfig } from '../services/ConfigService.ts';

const startupConfig = JSON.parse(readFileSync(new URL(
  '../../crates/pantograph-runtime-registry/tests/fixtures/startup_shared_resource_config.json',
  import.meta.url,
), 'utf8')) as Partial<AppConfig>;
const defaultConfig: AppConfig = {
  models: {
    vlm_model_path: null, vlm_mmproj_path: null,
    embedding_model_path: null, candle_embedding_model_path: null,
  },
  device: { device: 'auto', gpu_layers: -1 },
  connection_mode: { type: 'None' },
  external_url: null,
  api_key: null,
};
// The startup fixture supplies the backing domains; config reads also return defaults.
const loadedConfig: AppConfig = { ...startupConfig, ...defaultConfig };
const expectedSave: AppConfig = {
  ...loadedConfig,
  connection_mode: { type: 'External', url: 'http://localhost:1234' },
  external_url: 'http://localhost:1234',
  api_key: null,
};

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
  let configState: { config: AppConfig; isLoading: boolean; error: string | null } = {
    config: defaultConfig, isLoading: false, error: null,
  };
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
          ? { config: defaultConfig, isLoading: false, error: 'malformed saved config' }
          : { config: loadedConfig, isLoading: false, error: null };
        subscriber?.(configState);
      },
      refreshServerMode: async () => {},
      saveConfig: async (config: unknown) => {
        // Normalize the VM payload as IPC would, then check all preserved settings.
        const payload: unknown = JSON.parse(JSON.stringify(config));
        assert.deepEqual(payload, expectedSave);
        saved.push(payload);
      },
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
  return { context, saved, connections, finishLoad, saveConfig: context.ConfigService.saveConfig };
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
  assert.deepEqual(f.saved[0], expectedSave);
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

test('the save boundary rejects malformed startup domains and incomplete config', async () => {
  const f = fixture();
  await assert.rejects(f.saveConfig({
    ...expectedSave,
    runtime_resource_domains: [{ id: 'shared-host', capacity_bytes: 100 }],
  }));
  for (const field of ['models', 'device', 'connection_mode', 'external_url', 'api_key']) {
    const incomplete: Record<string, unknown> = { ...expectedSave };
    delete incomplete[field];
    await assert.rejects(f.saveConfig(incomplete));
  }
  assert.deepEqual(f.saved, []);
});
