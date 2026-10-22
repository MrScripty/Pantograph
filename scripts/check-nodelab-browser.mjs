/* global document, getComputedStyle */
// Real NodeLab component -> loopback transport -> isolated Rust service -> filesystem.
// No Tauri startup, installed node, model or GPU claim. Build nodelab_host first.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { createInterface } from 'node:readline';
import { mkdtemp, readFile, writeFile, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { evaluateBrowserFunction } from './cdp-browser-function.mjs';

const repo = fileURLToPath(new URL('../', import.meta.url));
const fixture = await mkdtemp(path.join(tmpdir(), 'pantograph-nodelab-browser-'));
const hostBinary = process.env.NODELAB_HOST_BINARY;
assert.ok(hostBinary, 'NODELAB_HOST_BINARY must name the built nodelab_host example');
const hosts = [];
const servers = [];
let browser;
let socket;
const commands = [];
const sessions = [];
try {
  await symlink(path.join(repo, 'node_modules'), path.join(fixture, 'node_modules'));
  await writeFile(path.join(fixture, 'index.html'), '<div id="app"></div><script type="module" src="/main.js"></script>');
  await writeFile(path.join(fixture, 'main.js'), `import { mount } from 'svelte'; import Page from ${JSON.stringify(path.join(repo, 'src/components/workbench/NodeLabPage.svelte'))}; mount(Page, { target: document.getElementById('app') });`);
  await build({ configFile: false, root: fixture, plugins: [svelte()], build: { outDir: 'dist', emptyOutDir: true } });
  function rustHost(empty) {
    const child = spawn(hostBinary, empty ? ['--empty-catalog'] : [], { stdio: ['pipe', 'pipe', 'inherit'] });
    hosts.push(child);
    const lines = createInterface({ input: child.stdout });
    let tail = Promise.resolve();
    return (command, args) => {
      const result = tail.then(() => new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { lines.removeListener('line', received); reject(new Error('Rust host command timeout')); }, 10000);
        const received = line => { clearTimeout(timeout); try { resolve(JSON.parse(line)); } catch (error) { reject(error); } };
        lines.once('line', received);
        child.stdin.write(JSON.stringify({ command, args }) + '\n');
      }));
      tail = result.catch(() => {});
      return result;
    };
  }
  const realHost = rustHost(false);
  const emptyHost = rustHost(true);
  async function startServer(mode) {
    const server = createServer(async (request, response) => {
      try {
        const url = new URL(request.url, 'http://127.0.0.1');
        if (url.pathname.startsWith('/api/workflow/commands/')) {
          if (request.method !== 'POST') { response.writeHead(405).end(); return; }
          let body = ''; for await (const chunk of request) { body += chunk; if (body.length > 100000) throw new Error('request too large'); }
          const command = decodeURIComponent(url.pathname.split('/').at(-1));
          const args = JSON.parse(body);
          commands.push(command);
          const result = mode === 'error' ? { ok: false, error: { code: 'runtime_not_ready', message: 'Qualification host unavailable' } } : await (mode === 'empty' ? emptyHost : realHost)(command, args);
          if (result.ok && command === 'create_custom_node_draft') sessions.push(result.value.session_id);
          response.writeHead(result.ok ? 200 : 400, { 'Content-Type': 'application/json' }).end(JSON.stringify(result.ok ? result.value : result.error));
          return;
        }
        const relative = url.pathname === '/' ? 'index.html' : url.pathname.slice(1);
        const file = path.resolve(fixture, 'dist', relative);
        if (!file.startsWith(path.resolve(fixture, 'dist') + path.sep)) { response.writeHead(403).end(); return; }
        response.setHeader('Content-Type', file.endsWith('.js') ? 'application/javascript' : 'text/html');
        response.end(await readFile(file));
      } catch (error) { response.writeHead(500, { 'Content-Type': 'application/json' }).end(JSON.stringify({ code: 'internal_error', message: String(error) })); }
    });
    servers.push(server);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    return `http://127.0.0.1:${server.address().port}`;
  }
  const realOrigin = await startServer('real');
  const emptyOrigin = await startServer('empty');
  const errorOrigin = await startServer('error');
  const profile = path.join(fixture, 'profile');
  // This disposable container cannot provide the Chromium user-namespace sandbox.
  // These verification flags do not modify the normal application launcher.
  browser = spawn(process.env.CHROMIUM_BINARY || 'chromium', ['--headless=new', '--no-sandbox', '--disable-gpu', '--disable-background-networking', '--no-first-run', '--no-default-browser-check', '--user-data-dir=' + profile, '--remote-debugging-port=0', 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'], env: { ...process.env, XDG_CONFIG_HOME: path.join(fixture, 'config'), XDG_CACHE_HOME: path.join(fixture, 'cache') } });
  let diagnostics = ''; browser.stderr.on('data', chunk => { diagnostics = (diagnostics + chunk).slice(-16384); });
  let port;
  const deadline = Date.now() + 10000;
  while (!port && Date.now() < deadline) {
    try { port = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]; }
    catch { await new Promise(resolve => setTimeout(resolve, 50)); }
  }
  assert.ok(port, 'Chromium startup unavailable: ' + diagnostics);
  const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(pages.find(page => page.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  let sequence = 0; const pending = new Map();
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data); const item = pending.get(message.id); if (!item) return;
    pending.delete(message.id); clearTimeout(item.timer);
    if (message.error) item.reject(new Error(JSON.stringify(message.error))); else item.resolve(message.result);
  });
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(method + ' timeout')); }, 15000);
    pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = (callback, args) => evaluateBrowserFunction(call, callback, args);
  const text = () => evaluate(() => document.body.innerText);
  async function waitText(expected) {
    const deadline = Date.now() + 10000;
    while (Date.now() < deadline) { if ((await text()).includes(expected)) return; await new Promise(resolve => setTimeout(resolve, 30)); }
    assert.fail(`Missing UI text: ${expected}\n${await text()}\nCommands: ${JSON.stringify(commands)}`);
  }
  async function navigate(origin) {
    await call('Page.navigate', { url: `${origin}/?workflowHost=${encodeURIComponent(origin)}` });
    await waitText('NodeLab');
  }
  async function clickButton(label) {
    const rect = await evaluate(label => {
      const button = [...document.querySelectorAll('button')].find(item => item.textContent.trim() === label);
      if (!button || button.disabled) throw new Error('Button unavailable: ' + label);
      button.scrollIntoView(); const rect = button.getBoundingClientRect(); return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    }, [label]);
    await call('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...rect });
    await call('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...rect });
  }
  async function enter(selector, value) {
    await evaluate(selector => { const input = document.querySelector(selector); input.focus(); }, [selector]);
    await call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'a', code: 'KeyA', modifiers: 2, windowsVirtualKeyCode: 65 });
    await call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', code: 'KeyA', modifiers: 2, windowsVirtualKeyCode: 65 });
    await call('Input.insertText', { text: value });
  }
  await navigate(emptyOrigin); await waitText('This host has no admitted custom nodes');
  assert.equal(await evaluate(() => [...document.querySelectorAll('button')].some(button => button.textContent.startsWith('Create '))), false);
  await navigate(errorOrigin); await waitText('Qualification host unavailable');
  assert.equal(await evaluate(() => document.querySelector('[role="alert"]')?.textContent), 'Qualification host unavailable');
  await navigate(realOrigin); await waitText('Create Host uppercase draft');
  // Tab/Enter activation proves keyboard access to creation, followed by real pointer actions.
  await evaluate(() => document.activeElement?.blur());
  for (let count = 0; count < 10; count++) {
    await call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    await call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    if (await evaluate(() => document.activeElement?.textContent === 'Create Host uppercase draft')) break;
  }
  assert.equal(await evaluate(() => document.activeElement?.textContent), 'Create Host uppercase draft');
  assert.equal(await evaluate(() => getComputedStyle(document.activeElement).outlineStyle !== 'none'), true);
  await call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r', unmodifiedText: '\r' });
  await call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  await waitText('Graph contract valid');
  await enter('textarea', 'Saved Mixed case'); await clickButton('Apply input'); await waitText('Input saved in draft');
  await clickButton('Save workflow'); await waitText('Workflow saved');
  const savedPath = await evaluate(() => document.querySelector('code')?.textContent);
  assert.ok(savedPath);
  const file = (await realHost('load_workflow', { path: savedPath })).value;
  assert.equal(file.graph.nodes.find(node => node.node_type === 'text-input').data.text, 'Saved Mixed case');
  const pin = file.graph.nodes.find(node => node.id === 'custom').data.custom_node_identity;
  await enter('input', savedPath); await clickButton('Reopen saved workflow'); await waitText('Graph contract valid');
  assert.equal(await evaluate(() => document.querySelector('textarea')?.value), 'Saved Mixed case');
  const reloaded = (await realHost('load_workflow', { path: savedPath })).value;
  assert.deepEqual(reloaded.graph.nodes.find(node => node.id === 'custom').data.custom_node_identity, pin);
  assert.equal((await realHost('get_node_lab_draft', { executionId: sessions[0] })).error.code, 'session_not_found');
  // Same saved graph opened on a host without its implementation stays visible and blocked.
  await emptyHost('save_workflow', { name: 'stale', graph: file.graph });
  await navigate(emptyOrigin); await waitText('This host has no admitted custom nodes');
  await enter('input', '.pantograph/workflows/stale.json'); await clickButton('Reopen saved workflow'); await waitText('Blocks execution:');
  assert.ok((await text()).includes('host-uppercase'));
  assert.ok(!commands.some(command => command.includes('run_execution') || command.includes('register') || command.includes('publish')));
  console.log(JSON.stringify({ ok: true, claims: ['real empty catalog', 'transport failure distinct from empty', 'keyboard creation and visible focus', 'pointer edit/save/reopen through Rust filesystem', 'exact pin preserved', 'replaced session closed', 'missing implementation visible and blocking'], mode: 'headless Chromium, disposable profile, no sandbox, GPU disabled', nativeTauri: 'not qualified' }, null, 2));
} finally {
  socket?.close();
  if (browser && browser.exitCode === null) { browser.kill('SIGTERM'); await new Promise(resolve => { browser.once('exit', resolve); setTimeout(resolve, 2000).unref(); }); }
  for (const server of servers) await new Promise(resolve => server.close(resolve));
  for (const host of hosts) { host.stdin.end(); if (host.exitCode === null) await new Promise(resolve => { host.once('exit', resolve); setTimeout(() => { host.kill('SIGTERM'); resolve(); }, 2000).unref(); }); }
  await rm(fixture, { recursive: true, force: true });
}
