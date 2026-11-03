import assert from 'node:assert/strict';
import test, { beforeEach, afterEach } from 'node:test';
import { registerHooks } from 'node:module';
import { environment } from './test-support/importEnvironment.ts';
import { clearValidationCache } from './ValidationCache.ts';
import type { ComponentUpdate, ImportResult } from '../types.ts';

// Scoped resolution substitutes platform effects, not the admission algorithm.
const hooks = registerHooks({
  resolve(specifier, context, nextResolve) {
    if (context.parentURL?.endsWith('/ImportManager.ts') &&
        (specifier === '@tauri-apps/api/core' || specifier === './GlobRegistry')) {
      return { url: new URL('./test-support/importEnvironment.ts', import.meta.url).href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
});
const { ImportManager } = await import('./ImportManager.ts');
const { ComponentRegistry } = await import('./ComponentRegistry.ts');
hooks.deregister();

const logger = { log() {} };
const originalFetch = globalThis.fetch;
let content: string;
let imports: number;
let validations: number;
function AcceptedComponent() {}
function ReplacementComponent() {}
const fullPath = '/src/generated/Fixture.svelte';

beforeEach(() => {
  clearValidationCache();
  content = '<div>safe fixture</div>';
  imports = 0;
  validations = 0;
  environment.lookups = 0;
  environment.invoke = async (command, args) => {
    validations++;
    assert.equal(command, 'validate_component');
    assert.deepEqual(args, { relativePath: fullPath });
    return { valid: true, error: null };
  };
  environment.modules = {
    [fullPath]: async () => { imports++; return { default: AcceptedComponent }; },
  };
  globalThis.fetch = async () => new Response(content);
});
afterEach(() => { globalThis.fetch = originalFetch; clearValidationCache(); });

function failed(result: ImportResult, failureCode: string): void {
  assert.equal(result.success, false);
  assert.equal(result.component, null);
  assert.ok(!result.success);
  assert.equal(result.failureCode, failureCode);
  assert.ok(result.error.length > 0);
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(r => { resolve = r; });
  return { promise, resolve };
}
function update(overrides: Partial<ComponentUpdate> = {}): ComponentUpdate {
  return {
    id: 'fixture', path: 'Fixture.svelte', source: content,
    position: { x: 1, y: 2 }, size: { width: 30, height: 40 }, ...overrides,
  };
}

for (const mode of ['sync-throw', 'async-reject', 'missing-backend'] as const) {
  test(`validation ${mode} fails closed before any importer lookup`, async () => {
    environment.invoke = () => {
      if (mode === 'sync-throw') throw new Error('controlled synchronous failure');
      return Promise.reject(mode === 'missing-backend' ? 'Tauri unavailable' : new Error('controlled rejection'));
    };
    const manager = new ImportManager({ logger });
    const result = await manager.importComponent('Fixture.svelte');
    failed(result, 'validation-unavailable');
    assert.equal(imports, 0);
    assert.equal(environment.lookups, 0);
    assert.equal(manager.isCached('Fixture.svelte'), false);
  });
}

for (const response of [null, {}, { valid: 'true' }, { valid: 1 }, { valid: true, error: 42 }, []]) {
  test(`malformed validation response ${JSON.stringify(response)} fails closed`, async () => {
    environment.invoke = async () => response;
    failed(await new ImportManager({ logger }).importComponent('Fixture.svelte'), 'validation-response-invalid');
    assert.equal(imports, 0);
    assert.equal(environment.lookups, 0);
  });
}

test('source HTTP failure, fetch rejection and body rejection never reach validator or importer', async () => {
  for (const fetcher of [
    async () => new Response('', { status: 503 }),
    async () => { throw new Error('offline'); },
    async () => { const response = new Response(''); response.text = async () => { throw new Error('body failure'); }; return response; },
  ]) {
    globalThis.fetch = fetcher;
    failed(await new ImportManager({ logger }).importComponent('Fixture.svelte'), 'validation-unavailable');
  }
  assert.equal(validations, 0);
  assert.equal(imports, 0);
  assert.equal(environment.lookups, 0);
});

test('explicit invalid result is retained for unchanged source and never imports', async () => {
  environment.invoke = async () => { validations++; return { valid: false, error: 'controlled invalid component' }; };
  const manager = new ImportManager({ logger });
  for (let attempt = 0; attempt < 2; attempt++) {
    const result = await manager.importComponent('Fixture.svelte');
    failed(result, 'validation-invalid');
    assert.equal(result.error, 'controlled invalid component');
  }
  assert.equal(validations, 1);
  assert.equal(imports, 0);
});

test('genuine valid result imports and caches the actual component', async () => {
  const manager = new ImportManager({ logger });
  const result = await manager.importComponent('Fixture.svelte');
  assert.equal(result.success, true);
  assert.equal(result.component, AcceptedComponent);
  assert.equal(await manager.importComponent('Fixture.svelte'), result);
  assert.equal(validations, 1);
  assert.equal(imports, 1);
});

test('transport and malformed failures are not cached; explicit retry validates again', async () => {
  for (const bad of [() => Promise.reject('offline'), async () => ({ valid: 'true' })]) {
    clearValidationCache();
    const manager = new ImportManager({ logger });
    environment.invoke = bad;
    assert.equal((await manager.importComponent('Fixture.svelte')).success, false);
    environment.invoke = async () => { validations++; return { valid: true, error: null }; };
    assert.equal((await manager.importComponent('Fixture.svelte')).success, true);
  }
  assert.equal(validations, 2);
  assert.equal(imports, 2);
});

test('concurrent requests for the same candidate share validation and import', async () => {
  const gate = deferred<unknown>();
  environment.invoke = () => { validations++; return gate.promise; };
  const manager = new ImportManager({ logger });
  const first = manager.importComponent('Fixture.svelte');
  const second = manager.importComponent('Fixture.svelte');
  gate.resolve({ valid: true, error: null });
  assert.equal(await first, await second);
  assert.equal(validations, 1);
  assert.equal(imports, 1);
});

test('replacement revokes old validation admission without deleting the newer pending request', async () => {
  const started = deferred<void>();
  const firstGate = deferred<unknown>();
  const nextGate = deferred<unknown>();
  environment.invoke = () => { validations++; started.resolve(); return validations === 1 ? firstGate.promise : nextGate.promise; };
  const manager = new ImportManager({ logger });
  const first = manager.importComponent('Fixture.svelte');
  await started.promise;
  content = '<div>new candidate</div>';
  const second = manager.reimportComponent('Fixture.svelte');
  firstGate.resolve({ valid: true, error: null });
  failed(await first, 'superseded');
  const third = manager.importComponent('Fixture.svelte');
  nextGate.resolve({ valid: true, error: null });
  assert.equal(await second, await third);
  assert.equal(validations, 2);
  assert.equal(imports, 1);
});

test('initial registration failure reaches error state and explicit retry recovers', async () => {
  environment.invoke = async () => { throw new Error('offline'); };
  const registry = new ComponentRegistry({ logger });
  await registry.registerFromUpdate(update());
  assert.equal(registry.getById('fixture')?.status, 'error');
  assert.equal(registry.getById('fixture')?.component, null);
  assert.equal(registry.getErrorReporter().getLatestError('fixture')?.errorType, 'validation');
  environment.invoke = async () => ({ valid: true, error: null });
  await registry.retry('fixture');
  assert.equal(registry.getById('fixture')?.status, 'ready');
  assert.equal(registry.getById('fixture')?.component, AcceptedComponent);
  assert.equal(registry.getErrorReporter().hasError('fixture'), false);
});

test('failed replacement preserves accepted UI metadata; retry uses the failed candidate', async () => {
  const registry = new ComponentRegistry({ logger });
  await registry.registerFromUpdate(update());
  const accepted = registry.getById('fixture')!;
  const candidate = update({ path: 'Replacement.svelte', source: '<div>replacement</div>', position: { x: 5, y: 6 } });
  environment.invoke = async () => { throw new Error('offline'); };
  await registry.registerFromUpdate(candidate);
  const retained = registry.getById('fixture')!;
  assert.equal(retained.status, 'ready');
  assert.equal(retained.component, accepted.component);
  assert.equal(retained.source, accepted.source);
  assert.equal(retained.path, accepted.path);
  assert.deepEqual(retained.position, accepted.position);
  assert.equal(retained.error, undefined);
  assert.equal(retained.pendingUpdate?.status, 'error');
  assert.equal(imports, 1);
  environment.invoke = async (_command, args) => {
    assert.deepEqual(args, { relativePath: '/src/generated/Replacement.svelte' });
    return { valid: true, error: null };
  };
  environment.modules['/src/generated/Replacement.svelte'] = async () => ({ default: ReplacementComponent });
  await registry.retry('fixture');
  assert.equal(registry.getById('fixture')?.component, ReplacementComponent);
  assert.equal(registry.getById('fixture')?.path, candidate.path);
  assert.equal(registry.getById('fixture')?.source, candidate.source);
  assert.equal(registry.getById('fixture')?.pendingUpdate, undefined);
});

test('HMR failure keeps accepted component and refreshById reports failed replacement', async () => {
  const registry = new ComponentRegistry({ logger });
  await registry.registerFromUpdate(update());
  content = '<div>changed</div>';
  environment.invoke = async () => ({ valid: false, error: 'invalid update' });
  await registry.refreshByPaths([fullPath]);
  assert.equal(registry.getById('fixture')?.component, AcceptedComponent);
  assert.equal(registry.getById('fixture')?.status, 'ready');
  assert.equal(registry.getById('fixture')?.pendingUpdate?.status, 'error');
  assert.equal(await registry.refreshById('fixture'), false);
  assert.equal(imports, 1);
});

test('newer registration wins over an older validation completion', async () => {
  const gate = deferred<unknown>();
  const started = deferred<void>();
  environment.invoke = (_command, args) => {
    if ((args as { relativePath: string }).relativePath === fullPath) { started.resolve(); return gate.promise; }
    return Promise.resolve({ valid: true, error: null });
  };
  environment.modules['/src/generated/New.svelte'] = async () => ({ default: ReplacementComponent });
  const registry = new ComponentRegistry({ logger });
  const old = registry.registerFromUpdate(update());
  await started.promise;
  await registry.registerFromUpdate(update({ path: 'New.svelte', source: 'new' }));
  gate.resolve({ valid: false, error: 'stale failure' });
  await old;
  assert.equal(registry.getById('fixture')?.component, ReplacementComponent);
  assert.equal(registry.getById('fixture')?.source, 'new');
  assert.equal(registry.getErrorReporter().hasError('fixture'), false);
});

for (const removal of ['unregister', 'clear'] as const) {
  test(`${removal} during validation cannot resurrect a component`, async () => {
    const gate = deferred<unknown>();
    const started = deferred<void>();
    environment.invoke = () => { started.resolve(); return gate.promise; };
    const registry = new ComponentRegistry({ logger });
    const pending = registry.registerFromUpdate(update());
    await started.promise;
    if (removal === 'unregister') registry.unregister('fixture');
    else registry.clear();
    gate.resolve({ valid: true, error: null });
    await pending;
    assert.equal(registry.getAll().length, 0);
    assert.equal(registry.getErrorReporter().hasError('fixture'), false);
  });
}

test('changed source is revalidated after explicit invalid rejection', async () => {
  environment.invoke = async () => { validations++; return { valid: false, error: null }; };
  const manager = new ImportManager({ logger });
  failed(await manager.importComponent('Fixture.svelte'), 'validation-invalid');
  content = '<div>corrected component</div>';
  environment.invoke = async () => { validations++; return { valid: true }; };
  assert.equal((await manager.importComponent('Fixture.svelte')).success, true);
  assert.equal(validations, 2);
  assert.equal(imports, 1);
});

test('late importer success cannot overwrite the cache after replacement', async () => {
  const oldImport = deferred<unknown>();
  const started = deferred<void>();
  environment.modules[fullPath] = () => { started.resolve(); return oldImport.promise; };
  const manager = new ImportManager({ logger });
  const old = manager.importComponent('Fixture.svelte');
  await started.promise;
  content = '<div>new import</div>';
  environment.modules[fullPath] = async () => ({ default: ReplacementComponent });
  assert.equal((await manager.reimportComponent('Fixture.svelte')).component, ReplacementComponent);
  oldImport.resolve({ default: AcceptedComponent });
  failed(await old, 'superseded');
  assert.equal((await manager.importComponent('Fixture.svelte')).component, ReplacementComponent);
});

test('accepted component stays ready throughout pending replacement and failed candidate HMR recovers', async () => {
  const registry = new ComponentRegistry({ logger });
  await registry.registerFromUpdate(update());
  const gate = deferred<unknown>();
  environment.invoke = () => gate.promise;
  const snapshots: Array<{ component: unknown; status: string; pending: string | undefined }> = [];
  const unsubscribe = registry.subscribe(comps => {
    const comp = comps[0];
    if (comp) snapshots.push({ component: comp.component, status: comp.status, pending: comp.pendingUpdate?.status });
  });
  const pending = registry.registerFromUpdate(update({ path: 'Other.svelte' }));
  gate.resolve({ valid: false, error: 'invalid candidate' });
  await pending;
  assert.ok(snapshots.some(s => s.pending === 'loading'));
  assert.ok(snapshots.some(s => s.pending === 'error'));
  assert.ok(snapshots.every(s => s.component === AcceptedComponent && s.status === 'ready'));
  assert.equal(registry.getErrored().length, 1);
  content = '<div>fixed candidate</div>';
  environment.invoke = async () => ({ valid: true, error: null });
  environment.modules['/src/generated/Other.svelte'] = async () => ({ default: ReplacementComponent });
  await registry.refreshByPaths(['/src/generated/Other.svelte']);
  assert.equal(registry.getById('fixture')?.component, ReplacementComponent);
  assert.equal(registry.getById('fixture')?.pendingUpdate, undefined);
  unsubscribe();
});

test('different registry IDs sharing an initial path do not cancel one another', async () => {
  const registry = new ComponentRegistry({ logger });
  await Promise.all([
    registry.registerFromUpdate(update()),
    registry.registerFromUpdate(update({ id: 'second' })),
  ]);
  for (const component of registry.getAll()) {
    assert.equal(component.status, 'ready');
    assert.equal(component.component, AcceptedComponent);
  }
  assert.equal(imports, 1);
});

for (const interruption of ['unregister', 'clear', 'replace'] as const) {
  test(`queued HMR cannot revive or overwrite an entry after ${interruption}`, async () => {
    const registry = new ComponentRegistry({ logger });
    await registry.registerFromUpdate(update());
    await registry.registerFromUpdate(update({ id: 'second' }));
    content = 'HMR candidate';
    const gate = deferred<unknown>();
    const started = deferred<void>();
    environment.invoke = () => { started.resolve(); return gate.promise; };
    const batch = registry.refreshByPaths([fullPath]);
    await started.promise;
    if (interruption === 'clear') registry.clear();
    else if (interruption === 'unregister') registry.unregister('second');
    else {
      environment.modules['/src/generated/New.svelte'] = async () => ({ default: ReplacementComponent });
      environment.invoke = async () => ({ valid: true, error: null });
      await registry.registerFromUpdate(update({ id: 'second', path: 'New.svelte', source: 'newer' }));
    }
    gate.resolve({ valid: true, error: null });
    await batch;
    if (interruption === 'clear') assert.deepEqual(registry.getAll(), []);
    else if (interruption === 'unregister') assert.equal(registry.getById('second'), undefined);
    else {
      assert.equal(registry.getById('second')?.component, ReplacementComponent);
      assert.equal(registry.getById('second')?.source, 'newer');
    }
  });
}

for (const operation of ['refresh', 'replace'] as const) {
  test(`geometry edits after ${operation} begins win over its starting geometry`, async () => {
    const registry = new ComponentRegistry({ logger });
    await registry.registerFromUpdate(update());
    content = 'new source';
    const gate = deferred<unknown>();
    const started = deferred<void>();
    environment.invoke = () => { started.resolve(); return gate.promise; };
    const pending = operation === 'refresh'
      ? registry.refreshByPaths([fullPath])
      : registry.registerFromUpdate(update({ position: { x: 10, y: 20 }, size: { width: 50, height: 60 } }));
    await started.promise;
    registry.updatePosition('fixture', 100, 200);
    registry.updateSize('fixture', 300, 400);
    gate.resolve({ valid: true, error: null });
    await pending;
    const component = registry.getById('fixture')!;
    assert.deepEqual(component.position, { x: 100, y: 200 });
    assert.deepEqual(component.size, { width: 300, height: 400 });
    assert.equal(component.props?.style, 'position: absolute; left: 100px; top: 200px; width: 300px; height: 400px;');
  });
}

for (const latestValid of [true, false]) {
  test(`one ID's refresh lets another live ID join the newest shared-path result (valid=${latestValid})`, async () => {
    const registry = new ComponentRegistry({ logger });
    await registry.registerFromUpdate(update());
    content = 'held source';
    const oldValidation = deferred<unknown>();
    const started = deferred<void>();
    environment.invoke = () => { validations++; started.resolve(); return oldValidation.promise; };
    const firstRefresh = registry.refreshById('fixture');
    await started.promise;
    const other = registry.registerFromUpdate(update({ id: 'second' }));
    content = 'latest source';
    environment.invoke = async () => { validations++; return { valid: latestValid, error: latestValid ? null : 'latest invalid' }; };
    environment.modules[fullPath] = async () => { imports++; return { default: ReplacementComponent }; };
    assert.equal(await registry.refreshById('fixture'), latestValid);
    oldValidation.resolve({ valid: true, error: null });
    assert.equal(await firstRefresh, false);
    await other;
    assert.equal(validations, 3); // initial, held and explicitly requested replacement only
    if (latestValid) {
      assert.equal(registry.getById('second')?.component, ReplacementComponent);
      assert.equal(registry.getById('second')?.status, 'ready');
      assert.equal(registry.getErrorReporter().hasError('second'), false);
    } else {
      assert.equal(registry.getById('second')?.status, 'error');
      assert.equal(registry.getById('second')?.error, 'latest invalid');
      assert.equal(imports, 1);
    }
  });
}
