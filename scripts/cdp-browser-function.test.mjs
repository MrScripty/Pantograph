import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { Session } from 'node:inspector/promises';
import { evaluateBrowserFunction } from './cdp-browser-function.mjs';

test('real local V8 inspector preserves hostile text through the CDP argument boundary', async () => {
  const session = new Session();
  session.connect();
  try {
    const payload = { text: '"); throw new Error("injected"); //\n</script>\u2028\u2029', nested: { quote: "'\\`" } };
    const result = await evaluateBrowserFunction((method, params) => session.post(method, params), async value => value, [payload]);
    assert.deepEqual(result, payload);
  } finally {
    session.disconnect();
  }
});

test('executable-looking runtime data travels separately from callback source', async () => {
  const context = vm.createContext({});
  const calls = [];
  const call = async (method, params) => {
    calls.push({ method, params });
    if (method === 'Runtime.evaluate') {
      assert.equal(params.expression, 'globalThis');
      return { result: { objectId: 'page-1' } };
    }
    if (method === 'Runtime.callFunctionOn') {
      const callback = vm.runInContext(`(${params.functionDeclaration})`, context);
      // CDP returns JSON values, not objects with a remote realm's prototype.
      return { result: { value: JSON.parse(JSON.stringify(await callback(...params.arguments.map(arg => arg.value)))) } };
    }
    assert.equal(method, 'Runtime.releaseObject');
    return {};
  };
  const payload = { text: '"); globalThis.injected = true; //\n</script><script>bad()</script>\u2028\u2029', graph: { nodes: [{ id: '${globalThis.injected=true}' }] } };
  const callback = async value => ({ text: value.text, graph: value.graph });
  callback.toString = () => '() => { globalThis.injected = true; }';
  assert.deepEqual(await evaluateBrowserFunction(call, callback, [payload]), payload);
  assert.equal(context.injected, undefined);
  const invocation = calls.find(item => item.method === 'Runtime.callFunctionOn').params;
  assert.ok(!invocation.functionDeclaration.includes(payload.text));
  assert.deepEqual(invocation.arguments, [{ value: payload }]);
  assert.deepEqual(calls.at(-1), { method: 'Runtime.releaseObject', params: { objectId: 'page-1' } });
});

test('each invocation obtains the current navigation context and releases it on exception', async () => {
  let page = 0;
  const released = [];
  const call = async (method, params) => {
    if (method === 'Runtime.evaluate') return { result: { objectId: `page-${++page}` } };
    if (method === 'Runtime.releaseObject') { released.push(params.objectId); return {}; }
    assert.equal(params.objectId, `page-${page}`);
    return page === 1 ? { result: { value: 'first' } } : { exceptionDetails: { text: 'Navigation failure' } };
  };
  assert.equal(await evaluateBrowserFunction(call, () => 'first'), 'first');
  await assert.rejects(evaluateBrowserFunction(call, () => 'second'), /Navigation failure/);
  assert.deepEqual(released, ['page-1', 'page-2']);
});

test('source strings and missing execution contexts fail closed', async () => {
  await assert.rejects(evaluateBrowserFunction(() => assert.fail('must not call CDP'), 'alert(1)'), TypeError);
  await assert.rejects(evaluateBrowserFunction(async () => ({ result: {} }), () => null), /context unavailable/);
});
