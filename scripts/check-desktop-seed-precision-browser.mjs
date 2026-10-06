// Exercise the actual NumberInput component and persistence helpers in Chromium.
// The surrounding graph shell/store are controlled fixtures; no Tauri/model is loaded.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const repo = fileURLToPath(new URL('../', import.meta.url));
const fixture = await mkdtemp(path.join(tmpdir(), 'pantograph-seed-browser-'));
let server;
let browser;
let socket;
try {
  await symlink(path.join(repo, 'node_modules'), path.join(fixture, 'node_modules'));
  await writeFile(path.join(fixture, 'BaseNode.svelte'), `
    <script>let { header, children } = $props();</script>
    {@render header?.()}{@render children?.()}
  `);
  await writeFile(path.join(fixture, 'store.js'), `
    import { writable } from 'svelte/store';
    export const nodes = writable([]);
    export const edges = writable([]);
    export function updateNodeData(id, patch) {
      nodes.update(values => values.map(node => node.id === id
        ? { ...node, data: { ...node.data, ...patch } } : node));
    }
  `);
  await writeFile(path.join(fixture, 'Harness.svelte'), `
    <script>
      import NumberInputNode from ${JSON.stringify(path.join(repo, 'src/components/nodes/workflow/NumberInputNode.svelte'))};
      import { nodes } from './store.js';
    </script>
    <NumberInputNode id="number" data={$nodes[0].data} />
  `);
  await writeFile(path.join(fixture, 'main.js'), `
    import { mount, tick } from 'svelte';
    import { get } from 'svelte/store';
    import Harness from './Harness.svelte';
    import { nodes, edges } from './store.js';
    import { assertDesktopSeedInputs, DESKTOP_SEED_ERROR } from ${JSON.stringify(path.join(repo, 'src/components/nodes/workflow/primitiveInputMetadata.ts'))};
    const passed = [];
    const check = (condition, message) => { if (!condition) throw new Error(message); };
    const seedEdge = { source: 'number', sourceHandle: 'value', target: 'inference', targetHandle: 'seed' };
    const reset = async (value, connected = true) => {
      nodes.set([{ id: 'number', type: 'number-input', data: value === undefined ? {} : { value } },
        { id: 'inference', type: 'llm-inference', data: {} }]);
      edges.set(connected ? [seedEdge] : []);
      await tick();
    };
    const graph = () => ({ nodes: get(nodes), edges: get(edges) });
    const input = () => document.querySelector('input');
    const enter = async text => {
      input().value = text;
      input().dispatchEvent(new Event('input', { bubbles: true }));
      await tick();
    };
    const replay = async () => {
      const saved = JSON.stringify(graph());
      const loaded = JSON.parse(saved);
      nodes.set(loaded.nodes);
      edges.set(loaded.edges);
      await tick();
      return loaded;
    };
    const assertRejected = loaded => {
      let rejected = false;
      try { assertDesktopSeedInputs(loaded); } catch (error) {
        rejected = error.message.includes(DESKTOP_SEED_ERROR);
      }
      check(rejected, 'Unsafe seed reached the submit boundary');
      check(input().getAttribute('aria-invalid') === 'true', 'Missing invalid state');
      check(document.querySelector('[role="alert"]')?.textContent === DESKTOP_SEED_ERROR,
        'Missing visible seed error');
    };
    try {
      await reset(undefined);
      mount(Harness, { target: document.querySelector('#app') });
      await tick();
      check(input().type === 'text' && input().inputMode === 'numeric', 'Seed field must retain raw text');
      for (const text of ['0', '9007199254740991']) {
        await enter(text);
        check(get(nodes)[0].data.value === Number(text), 'Safe seed changed before save');
        const loaded = await replay();
        assertDesktopSeedInputs(loaded);
        check(input().value === text, 'Safe seed changed on reload');
        passed.push('accepted/save/load/replay: ' + text);
      }
      for (const text of ['9007199254740992', '9007199254740993', '18446744073709551615',
        '9007199254740991.1', '-', '1e', '1e3', 'Infinity', 'abc']) {
        await enter(text);
        check(get(nodes)[0].data.value === text, 'Authored seed was rounded, sanitized or omitted: ' + text);
        check(input().value === text, 'Authored seed disappeared from input: ' + text);
        assertRejected(graph());
        for (let attempt = 0; attempt < 2; attempt++) {
          const loaded = await replay();
          check(loaded.nodes[0].data.value === text && input().value === text, 'Reload lost raw seed');
          assertRejected(loaded);
        }
        passed.push('rejected/verbatim/save/load/replay: ' + text);
      }
      await enter('');
      check(get(nodes)[0].data.value === null, 'Clearing seed must remain unset');
      assertDesktopSeedInputs(await replay());
      await reset(undefined);
      check(!Object.hasOwn((await replay()).nodes[0].data, 'value'), 'Omission acquired a seed');
      passed.push('clearing and omission');
      for (const value of [2 ** 53, Number('18446744073709551615')]) {
        await reset(value);
        assertRejected(await replay());
      }
      passed.push('saved unsafe numeric seeds');
      await reset(1.25, false);
      check(input().type === 'number', 'Generic numeric input type changed');
      await enter('-2.5');
      check(get(nodes)[0].data.value === -2.5, 'Generic floating point input changed');
      assertDesktopSeedInputs(await replay());
      passed.push('generic floats');
      document.querySelector('#result').textContent = JSON.stringify({ ok: true, passed });
    } catch (error) {
      document.querySelector('#result').textContent = JSON.stringify({ ok: false, error: error.stack, passed });
    }
  `);
  await writeFile(path.join(fixture, 'index.html'), '<div id="app"></div><pre id="result">pending</pre><script type="module" src="/main.js"></script>');
  await build({
    root: fixture,
    configFile: false,
    logLevel: 'warn',
    plugins: [
      {
        name: 'controlled-number-input-host',
        enforce: 'pre',
        resolveId(source, importer) {
          if (!importer?.endsWith('/NumberInputNode.svelte')) return;
          if (source === '../BaseNode.svelte') return path.join(fixture, 'BaseNode.svelte');
          if (source === '../../../stores/workflowStore') return path.join(fixture, 'store.js');
        },
      },
      svelte({ configFile: false }),
    ],
    build: { target: 'esnext', outDir: path.join(fixture, 'dist') },
  });
  server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, 'http://localhost').pathname;
      const file = path.join(fixture, 'dist', pathname === '/' ? 'index.html' : pathname);
      const body = await readFile(file);
      response.setHeader('Content-Type', file.endsWith('.js') ? 'application/javascript' : 'text/html');
      response.end(body);
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const profile = path.join(fixture, 'profile');
  browser = spawn(process.env.CHROMIUM_BINARY || 'chromium', [
    '--headless=new', '--no-sandbox', '--disable-gpu', '--disable-background-networking',
    '--no-first-run', '--no-default-browser-check',
    '--user-data-dir=' + profile, '--remote-debugging-port=0', 'about:blank',
  ], { stdio: ['ignore', 'ignore', 'pipe'], env: {
    ...process.env, XDG_CONFIG_HOME: path.join(fixture, 'config'), XDG_CACHE_HOME: path.join(fixture, 'cache'),
  } });
  let browserError;
  browser.on('error', error => { browserError = error; });
  let diagnostic = '';
  browser.stderr.on('data', chunk => { diagnostic += chunk; });
  let port;
  const deadline = Date.now() + 10000;
  while (!port && Date.now() < deadline) {
    if (browserError) throw browserError;
    try { port = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]; }
    catch { await new Promise(resolve => setTimeout(resolve, 50)); }
  }
  assert.ok(port, 'Chromium did not start: ' + diagnostic);
  const pages = await (await fetch('http://127.0.0.1:' + port + '/json/list')).json();
  socket = new WebSocket(pages.find(page => page.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    clearTimeout(request.timer);
    if (message.error) request.reject(new Error(JSON.stringify(message.error)));
    else request.resolve(message.result);
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(method + ' timed out')); }, 15000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
  await send('Page.navigate', { url: 'http://127.0.0.1:' + server.address().port });
  const evaluated = await send('Runtime.evaluate', {
    expression: `new Promise(resolve => {
      const check = () => {
        const text = document.querySelector('#result')?.textContent;
        if (text && text !== 'pending') resolve(text);
        else setTimeout(check, 20);
      };
      check();
    })`,
    awaitPromise: true,
    returnByValue: true,
  });
  assert.ok(!evaluated.exceptionDetails, JSON.stringify(evaluated));
  const result = JSON.parse(evaluated.result.value);
  assert.equal(result?.ok, true, JSON.stringify(result));
  console.log(JSON.stringify(result, null, 2));
} finally {
  socket?.close();
  if (browser && browser.exitCode === null) {
    browser.kill('SIGTERM');
    await new Promise(resolve => {
      browser.once('exit', resolve);
      setTimeout(resolve, 2000).unref();
    });
  }
  server?.closeAllConnections();
  if (server) await new Promise(resolve => server.close(resolve));
  await rm(fixture, { recursive: true, force: true });
}
