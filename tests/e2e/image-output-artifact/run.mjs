// Actual ImageOutputNode and decoder; controlled graph shell/store/artifact transport.
// CPU/browser presentation evidence only. No Tauri, inference, model, or GPU runtime.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { evaluateBrowserFunction } from '../../../scripts/cdp-browser-function.mjs';

const testDir = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(testDir, '../../..');
const scratch = await mkdtemp(path.join(tmpdir(), 'pantograph-image-output-'));
const noSandbox = process.env.PANTOGRAPH_IMAGE_BROWSER_NO_SANDBOX === '1';
const sandboxMode = noSandbox ? 'disabled-explicit-opt-in' : 'enabled';
let server;
let browser;
let socket;
let evaluate;
const passed = [];
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
try {
  await symlink(path.join(repo, 'node_modules'), path.join(scratch, 'node_modules'));
  await writeFile(path.join(scratch, 'main.js'), `import ${JSON.stringify(path.join(testDir, 'fixture.ts'))};`);
  await writeFile(path.join(scratch, 'index.html'), '<!doctype html><html><head><meta charset="utf-8"><title>Image Output artifact fixture</title><style>body{font:16px sans-serif;background:#111;color:#eee}#app{width:360px;margin:24px}.base-node{padding:12px}img{max-width:100%}button,select{margin:4px}</style></head><body><main id="app"></main><script type="module" src="/main.js"></script></body></html>');
  const buildResult = await build({
    root: scratch, configFile: false, logLevel: 'warn',
    plugins: [
      {
        name: 'controlled-image-output-host', enforce: 'pre',
        resolveId(source, importer) {
          if (!importer?.endsWith('/ImageOutputNode.svelte')) return;
          if (source === '../BaseNode.svelte') return path.join(testDir, 'BaseNode.svelte');
          if (source === '../../../stores/workflowStore') return path.join(testDir, 'store.ts');
          if (source === '../../../services/workflow/WorkflowService') return path.join(testDir, 'transport.ts');
        },
      },
      svelte({ configFile: false }),
    ],
    build: { target: 'esnext', outDir: path.join(scratch, 'dist') },
  });
  // Only build-owned names reach the filesystem. Requests perform exact Map lookups.
  const assets = new Map();
  const buildOutputs = Array.isArray(buildResult) ? buildResult : [buildResult];
  for (const output of buildOutputs.flatMap((result) => result.output)) {
    const body = await readFile(path.join(scratch, 'dist', output.fileName));
    const contentType = output.fileName.endsWith('.js') ? 'application/javascript'
      : output.fileName.endsWith('.css') ? 'text/css' : 'text/html';
    assets.set('/' + output.fileName, { body, contentType });
  }
  assert.ok(assets.has('/index.html'), 'Build did not produce the fixture index');
  assets.set('/', assets.get('/index.html'));
  server = createServer((request, response) => {
    try {
      const pathname = new URL(request.url, 'http://localhost').pathname;
      const asset = assets.get(pathname);
      if (!asset) return response.writeHead(404).end();
      response.setHeader('Content-Type', asset.contentType);
      response.end(asset.body);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  console.log('Chromium sandbox mode: ' + sandboxMode);
  const profile = path.join(scratch, 'profile');
  browser = spawn(process.env.CHROMIUM_BINARY || 'chromium', [
    ...(noSandbox ? ['--no-sandbox'] : []),
    '--headless=new', '--disable-gpu', '--disable-background-networking', '--no-first-run',
    '--no-default-browser-check', '--user-data-dir=' + profile, '--remote-debugging-port=0', 'about:blank',
  ], { stdio: ['ignore', 'ignore', 'pipe'], env: {
    ...process.env, XDG_CONFIG_HOME: path.join(scratch, 'config'), XDG_CACHE_HOME: path.join(scratch, 'cache'),
  } });
  let browserError;
  let diagnostic = '';
  browser.on('error', (error) => { browserError = error; });
  browser.stderr.on('data', (chunk) => { diagnostic += chunk; });
  let port;
  const launchDeadline = Date.now() + 10_000;
  while (!port && Date.now() < launchDeadline) {
    if (browserError) throw browserError;
    if (browser.exitCode !== null || browser.signalCode !== null) break;
    try { port = (await readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]; }
    catch { await delay(50); }
  }
  assert.ok(port, `Browser acceptance unavailable: Chromium did not start (sandbox mode: ${sandboxMode}). ` + diagnostic);
  const pages = await (await fetch('http://127.0.0.1:' + port + '/json/list')).json();
  socket = new WebSocket(pages.find((page) => page.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener('message', (event) => {
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
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(method + ' timed out')); }, 15_000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
  evaluate = (callback, args = []) => evaluateBrowserFunction(send, callback, args);
  const waitFor = async (callback, args = [], description = 'browser state') => {
    const deadline = Date.now() + 10_000;
    while (Date.now() < deadline) {
      if (await evaluate(callback, args)) return;
      await delay(20);
    }
    throw new Error('Timed out waiting for ' + description + ': ' + await evaluate(() => document.body.innerText));
  };
  const snapshot = () => evaluate(() => window.imageArtifactFixture.snapshot());
  const set = (image) => evaluate(async (value) => window.imageArtifactFixture.set(value), [image]);
  const preview = () => evaluate(() => {
    const image = document.querySelector('img[alt="Output"]');
    return image ? { src: image.src, width: image.naturalWidth, height: image.naturalHeight, complete: image.complete } : null;
  });
  const waitPreview = () => waitFor(() => {
    const image = document.querySelector('img[alt="Output"]');
    return image?.complete && image.naturalWidth > 0;
  }, [], 'decoded preview');
  const key = async (name, code, virtualKeyCode) => {
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: name, code, windowsVirtualKeyCode: virtualKeyCode });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: name, code, windowsVirtualKeyCode: virtualKeyCode });
  };
  const clickButton = async (text) => {
    const coordinates = await evaluate((name) => {
      const button = [...document.querySelectorAll('button')].find((element) => element.textContent.trim() === name);
      if (!button) throw new Error(`No button ${name}`);
      const bounds = button.getBoundingClientRect();
      return { x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 };
    }, [text]);
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...coordinates });
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...coordinates });
  };
  await send('Page.navigate', { url: 'http://127.0.0.1:' + server.address().port });
  await waitFor(() => window.imageArtifactFixture?.ready || document.body.dataset.fixtureError, [], 'fixture startup');
  assert.equal(await evaluate(() => document.body.dataset.fixtureError ?? null), null);
  const images = await evaluate(() => window.imageArtifactFixture.images);
  const downloadsDirectory = path.join(scratch, 'downloads');
  await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: downloadsDirectory });

  console.log('checking PNG/JPEG decoding and actual downloaded files');
  for (const id of ['png', 'jpeg']) {
    // Descriptor format is authoritative, including when the scheduler hint differs.
    await set({ artifact_id: id, media_type: 'application/octet-stream' });
    await waitPreview();
    const current = await preview();
    assert.deepEqual({ width: current.width, height: current.height }, { width: images[id].width, height: images[id].height });
    assert.ok(current.src.startsWith('blob:'), 'preview must use retained bytes in a blob URL');
    const before = await snapshot();
    const blob = before.created.find((entry) => entry.url === current.src);
    assert.equal(blob.mediaType, images[id].mediaType);
    assert.deepEqual(before.reads.at(-1), { artifact_id: id }, 'artifact read must request the complete body by ID');
    await clickButton('Download');
    await waitFor((count) => window.imageArtifactFixture.snapshot().downloads.length > count, [before.downloads.length]);
    const download = (await snapshot()).downloads.at(-1);
    assert.equal(download.filename, id === 'jpeg' ? 'output.jpg' : 'output.png');
    const downloadBlob = (await snapshot()).created.find((entry) => entry.url === download.url);
    assert.equal(downloadBlob.mediaType, images[id].mediaType);
    assert.deepEqual((await snapshot()).parentEvents, [], 'node control pointer events escaped into the parent shell');
    let downloaded;
    const deadline = Date.now() + 10_000;
    while (Date.now() < deadline && !downloaded) {
      try { downloaded = await readFile(path.join(downloadsDirectory, download.filename)); }
      catch { await delay(20); }
    }
    assert.ok(downloaded, 'browser did not finish downloading ' + download.filename);
    assert.deepEqual([...downloaded], images[id].body, 'downloaded bytes differ from the retained artifact');
    passed.push(`${id}: decoded ${current.width}x${current.height}; ${downloaded.length} bytes downloaded as ${download.filename} (${downloadBlob.mediaType})`);
  }

  console.log('checking keyboard expansion, Escape, and restored focus');
  // Tab reaches the native Expand button; CDP dispatches native keys rather than DOM click events.
  await evaluate(() => document.activeElement?.blur());
  for (let attempt = 0; attempt < 10; attempt++) {
    if (await evaluate(() => document.activeElement?.textContent.trim() === 'Expand')) break;
    await key('Tab', 'Tab', 9);
  }
  assert.equal(await evaluate(() => document.activeElement?.textContent.trim()), 'Expand');
  await key('Enter', 'Enter', 13);
  await waitFor(() => document.querySelector('dialog')?.open, [], 'modal open');
  assert.equal(await evaluate(() => document.activeElement?.textContent.trim()), 'Close');
  await waitFor(() => document.querySelector('dialog img')?.naturalWidth > 0, [], 'expanded image decode');
  assert.equal(await evaluate(() => document.querySelector('dialog img')?.naturalWidth), images.jpeg.width);
  await key('Escape', 'Escape', 27);
  await waitFor(() => !document.querySelector('dialog')?.open, [], 'Escape dismissal');
  assert.equal(await evaluate(() => document.activeElement?.textContent.trim()), 'Expand');
  await clickButton('Expand');
  await waitFor(() => document.querySelector('dialog')?.open);
  await clickButton('Close');
  await waitFor(() => !document.querySelector('dialog')?.open);
  assert.deepEqual((await snapshot()).parentEvents, [], 'Expand pointer events escaped into the parent shell');
  passed.push('native keyboard Expand/Enter, modal focus, Escape, restored Expand focus; pointer Expand/Close');

  console.log('checking replacement and clearing lifecycle');
  const oldUrl = (await preview()).src;
  await set({ artifact_id: 'png' });
  await waitPreview();
  assert.ok((await snapshot()).revoked.includes(oldUrl), 'replacement must revoke the old preview URL');
  await clickButton('Expand');
  await waitFor(() => document.querySelector('dialog')?.open);
  const pngUrl = (await preview()).src;
  await set(null);
  await waitFor(() => !document.querySelector('img[alt="Output"]') && !document.querySelector('dialog')?.open);
  assert.ok((await snapshot()).revoked.includes(pngUrl), 'clearing must revoke the preview URL');
  assert.ok((await evaluate(() => document.body.innerText)).includes('No image yet'));
  passed.push('changed/cleared reference removes preview, closes modal, and revokes owned URLs');

  console.log('checking malformed references and retained-artifact failures');
  for (const invalid of ['legacy-base64', {}, { artifact_id: '' }, { artifact_id: 7 }]) {
    const before = await snapshot();
    await set(invalid);
    await waitFor(() => Boolean(document.querySelector('[role="alert"]')));
    assert.equal(await evaluate(() => document.querySelector('[role="alert"]').textContent.trim()),
      typeof invalid === 'string' ? 'Image output requires one typed artifact reference.' : 'Invalid image artifact reference.');
    assert.equal(await preview(), null);
    const after = await snapshot();
    assert.equal(after.descriptors.length, before.descriptors.length, 'malformed refs must not reach transport');
    assert.equal(after.created.length, before.created.length, 'malformed refs must not create URLs');
  }
  const failures = {
    partial: 'Image artifact read is incomplete.',
    nonimage: 'Artifact is not an image.',
    unreadable: 'Image artifact is not readable.',
    'descriptor-id-mismatch': 'Image artifact descriptor does not match the result.',
    'read-id-mismatch': 'Image artifact read does not match the result.',
    'mime-mismatch': 'Image artifact media type does not match its descriptor.',
    'read-failure': 'fixture retained read failed',
    'bad-bytes': 'Image artifact could not be decoded.',
  };
  for (const [id, expectedError] of Object.entries(failures)) {
    const beforeFailure = await snapshot();
    await set({ artifact_id: id });
    await waitFor(() => Boolean(document.querySelector('[role="alert"]')), [], id + ' failure');
    const error = await evaluate(() => document.querySelector('[role="alert"]').textContent.trim());
    assert.equal(error, expectedError, id + ' did not reach the intended failure boundary');
    assert.equal(await preview(), null, id + ' left a stale/broken preview');
    assert.equal(await evaluate(() => [...document.querySelectorAll('button')].some((button) => button.textContent.trim() === 'Download' && !button.disabled)), false, id + ' still enabled download');
    const afterFailure = await snapshot();
    if (id === 'bad-bytes') {
      assert.equal(afterFailure.created.length, beforeFailure.created.length + 1, 'decode failure must reach the browser decoder');
      assert.ok(afterFailure.revoked.includes(afterFailure.created.at(-1).url), 'decode failure must release its URL');
    } else {
      assert.equal(afterFailure.created.length, beforeFailure.created.length, 'invalid artifact transport must be rejected before URL creation');
    }
    passed.push(`${id}: ${error}`);
  }
  passed.push('malformed scheduler refs rejected before transport');

  console.log('checking superseded read/descriptor success and failure');
  for (const [id, outcome] of [['defer-read-stale', 'resolve'], ['defer-read-failure', 'reject'], ['defer-descriptor-stale', 'resolve']]) {
    await set({ artifact_id: id });
    await waitFor((value) => window.imageArtifactFixture.snapshot().pending.includes(value), [id], id + ' pending');
    await set({ artifact_id: 'jpeg' });
    await waitPreview();
    const current = await preview();
    const before = await snapshot();
    await evaluate(async (value, action) => window.imageArtifactFixture[action](value), [id, outcome]);
    await delay(80);
    assert.equal((await preview()).src, current.src, 'superseded completion replaced the current image');
    const after = await snapshot();
    assert.equal(after.created.length, before.created.length, 'superseded completion created a URL');
    assert.equal(await evaluate(() => Boolean(document.querySelector('[role="alert"]'))), false, 'superseded failure leaked into current UI');
    passed.push(`${id}: superseded ${outcome} cannot publish`);
  }
  const beforeUnmount = await snapshot();
  const liveBeforeUnmount = beforeUnmount.created.filter((entry) => !beforeUnmount.revoked.includes(entry.url));
  await evaluate(async () => window.imageArtifactFixture.unmount());
  const afterUnmount = await snapshot();
  for (const { url } of liveBeforeUnmount) assert.ok(afterUnmount.revoked.includes(url), 'unmount did not revoke ' + url);
  passed.push('unmount revokes every live URL');
  // A fresh component scope exercises completion after unmount without browser-navigation races.
  await evaluate(async () => window.imageArtifactFixture.remount());
  await set({ artifact_id: 'defer-read-unmount' });
  await waitFor(() => window.imageArtifactFixture.snapshot().pending.includes('defer-read-unmount'));
  await evaluate(async () => window.imageArtifactFixture.unmount());
  const beforeCompletion = await snapshot();
  await evaluate(async () => window.imageArtifactFixture.resolve('defer-read-unmount'));
  await delay(80);
  assert.equal((await snapshot()).created.length, beforeCompletion.created.length, 'unmounted read created a URL');
  assert.equal(await preview(), null);
  passed.push('pending retained read completion after unmount cannot publish');
  const version = await send('Browser.getVersion');
  console.log(JSON.stringify({ ok: true, environment: {
    browser: version.product, sandboxMode, headless: true, gpuDisabled: true,
    scope: 'CPU browser presentation; controlled transport/shell/store; actual Svelte component and browser raster decoder/downloads',
  }, passed }, null, 2));
} finally {
  if (evaluate && socket?.readyState === WebSocket.OPEN) {
    await evaluate(async () => window.imageArtifactFixture?.unmount()).catch(() => {});
  }
  socket?.close();
  if (browser?.pid && browser.exitCode === null && browser.signalCode === null) {
    const exited = new Promise((resolve) => browser.once('exit', resolve));
    browser.kill('SIGTERM');
    await Promise.race([exited, delay(2_000)]);
    if (browser.exitCode === null && browser.signalCode === null) {
      browser.kill('SIGKILL');
      await exited;
    }
  }
  server?.closeAllConnections();
  if (server) await new Promise((resolve) => server.close(resolve));
  await rm(scratch, { recursive: true, force: true });
}
