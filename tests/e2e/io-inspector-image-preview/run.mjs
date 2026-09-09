import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { remote } from 'webdriverio';

const testDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(testDir, '../../..');
const host = '127.0.0.1';
let vitePort = 0;
const driverPort = 44000 + Math.floor(Math.random() * 1000);
const webKitDriver = '/usr/bin/WebKitWebDriver';
const miniBrowser = '/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/MiniBrowser';
const fixturePath = path.join(testDir, 'fixture.ts');
const virtualEntry = '\0d02-io-inspector-entry';
let server;
let driver;
let browser;
let driverOutput = '';

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForDriver() {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (driver.exitCode !== null) throw new Error(`WebKitWebDriver exited with ${driver.exitCode}`);
    try {
      const response = await fetch(`http://${host}:${driverPort}/status`);
      if (response.ok) return;
    } catch {}
    await delay(50);
  }
  throw new Error('WebKitWebDriver did not become ready');
}

function cardSelector(id) {
  return `[data-testid="io-artifact-card"][data-artifact-id="${id}"]`;
}

async function card(id) {
  const element = await browser.$(cardSelector(id));
  await element.waitForExist({ timeout: 10_000 });
  return element;
}

async function clickWithin(artifactId, selector) {
  const artifactCard = await card(artifactId);
  const target = await artifactCard.$(selector);
  await target.scrollIntoView();
  await target.click();
  return artifactCard;
}

async function waitForText(element, expected) {
  await browser.waitUntil(async () => (await element.getText()).includes(expected), {
    timeout: 10_000,
    timeoutMsg: `Expected text ${JSON.stringify(expected)}`,
  });
}

try {
  server = await createServer({
    root: repoRoot,
    appType: 'custom',
    logLevel: 'error',
    server: { host, port: 0, strictPort: false },
    plugins: [
      {
        name: 'd02-virtual-test-page',
        enforce: 'pre',
        configureServer(viteServer) {
          viteServer.middlewares.use((request, response, next) => {
            if (request.url !== '/') return next();
            response.setHeader('Content-Type', 'text/html; charset=utf-8');
            response.end('<!doctype html><html><body><div id="app"></div><script type="module" src="/@d02-entry"></script></body></html>');
          });
        },
        resolveId(source) {
          if (source === '/@d02-entry') return virtualEntry;
          if (source.endsWith('/WorkflowProjectionSubscriptionService')) return '\0d02-subscription-noop';
        },
        load(id) {
          if (id === virtualEntry) return `import ${JSON.stringify(fixturePath)};`;
          if (id === '\0d02-subscription-noop') {
            return 'export async function subscribeDiagnosticsProjectionInvalidations() { return () => {}; }';
          }
        },
      },
    ],
  });
  await server.listen();
  const viteAddress = server.httpServer.address();
  if (!viteAddress || typeof viteAddress === 'string') throw new Error('Vite did not expose a TCP address');
  vitePort = viteAddress.port;

  driver = spawn(webKitDriver, [`--port=${driverPort}`], {
    cwd: repoRoot,
    detached: true,
    env: { ...process.env, DISPLAY: process.env.DISPLAY || ':0.0' },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  driver.stdout.on('data', (chunk) => { driverOutput += chunk; });
  driver.stderr.on('data', (chunk) => { driverOutput += chunk; });
  await waitForDriver();

  browser = await remote({
    hostname: host,
    port: driverPort,
    path: '/',
    logLevel: 'error',
    capabilities: {
      'webkitgtk:browserOptions': { binary: miniBrowser, args: ['--automation'] },
    },
  });
  await browser.setWindowSize(1440, 1000);
  await browser.url(`http://${host}:${vitePort}/`);
  await browser.waitUntil(async () => browser.execute(() => Boolean(window.d02?.ready)), {
    timeout: 20_000,
    timeoutMsg: 'fixture did not become ready',
  });
  const fixtureError = await browser.execute(() => document.body.dataset.fixtureError ?? null);
  assert.equal(fixtureError, null, fixtureError ?? undefined);
  assert.ok(await browser.execute(() => window.d02.pngByteLength) > 65_536);

  console.log('checking valid image decode and full-body request');
  const validCard = await clickWithin('valid-image', '[data-testid="io-artifact-read-button"]');
  const validPreview = await validCard.$('[data-testid="io-artifact-image-preview"]');
  await validPreview.waitForExist({ timeout: 10_000 });
  const decoded = await browser.executeAsync((selector, done) => {
    const image = document.querySelector(selector);
    image.decode().then(() => done({ width: image.naturalWidth, height: image.naturalHeight }), (error) => done({ error: String(error) }));
  }, `${cardSelector('valid-image')} [data-testid="io-artifact-image-preview"]`);
  assert.deepEqual(decoded, { width: 320, height: 320 });
  let snapshot = await browser.execute(() => window.d02.snapshot());
  assert.deepEqual(snapshot.readRequests[0], { artifact_id: 'payload-valid-image' });
  assert.equal(snapshot.createdUrls.length, 1);

  await clickWithin('valid-image', '[data-testid="io-artifact-read-button"]');
  await browser.waitUntil(async () => (await browser.execute(() => window.d02.snapshot())).createdUrls.length === 2);
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.ok(snapshot.revokedUrls.includes(snapshot.createdUrls[0]), 'refresh must revoke replaced preview URL');

  console.log('checking detached and current image error identity');
  await browser.execute((selector) => {
    window.d02DetachedImage = document.querySelector(selector);
  }, `${cardSelector('valid-image')} [data-testid="io-artifact-image-preview"]`);
  console.log('  captured old image');
  await clickWithin('valid-image', '[data-testid="io-artifact-read-button"]');
  console.log('  replacement clicked');
  await browser.waitUntil(async () => (await browser.execute(() => window.d02.snapshot())).createdUrls.length === 3);
  console.log('  replacement installed');
  await browser.execute(() => window.d02DetachedImage.dispatchEvent(new Event('error')));
  console.log('  detached error dispatched');
  assert.equal(await browser.execute((selector) => Boolean(document.querySelector(selector)),
    `${cardSelector('valid-image')} [data-testid="io-artifact-image-preview"]`), true,
    'an error from a detached replaced image must not remove the current preview');

  await browser.execute(() => window.d02.deferValidRefresh());
  console.log('  refresh deferred');
  await browser.execute((selector) => document.querySelector(selector).click(),
    `${cardSelector('valid-image')} [data-testid="io-artifact-read-button"]`);
  console.log('  pending refresh clicked');
  const currentUrl = await browser.execute((selector) => document.querySelector(selector)?.src,
    `${cardSelector('valid-image')} [data-testid="io-artifact-image-preview"]`);
  console.log('  current URL captured');
  await browser.execute((selector) => document.querySelector(selector).dispatchEvent(new Event('error')),
    `${cardSelector('valid-image')} [data-testid="io-artifact-image-preview"]`);
  console.log('  current error dispatched');
  await browser.execute(() => window.d02.rejectValidRefresh());
  console.log('  pending refresh rejected');
  const validAfterPendingFailure = await card('valid-image');
  await waitForText(validAfterPendingFailure, 'fixture pending refresh failed');
  assert.equal(await validAfterPendingFailure.$('[data-testid="io-artifact-image-preview"]').isExisting(), false,
    'a matching image error must remove the broken preview while refresh is pending');
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.ok(snapshot.revokedUrls.includes(currentUrl), 'matching broken image URL was not revoked');

  console.log('checking malformed image error');
  const urlsBeforeMalformed = snapshot.createdUrls.length;
  const malformedCard = await clickWithin('malformed-image', '[data-testid="io-artifact-read-button"]');
  await waitForText(malformedCard, 'could not be decoded');
  assert.equal(await malformedCard.$('[data-testid="io-artifact-image-preview"]').isExisting(), false);
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.equal(snapshot.createdUrls.length, urlsBeforeMalformed + 1);
  assert.ok(snapshot.revokedUrls.includes(snapshot.createdUrls.at(-1)), 'decode failure must revoke its object URL');

  console.log('checking incomplete read and stream errors');
  const urlsBeforePartial = snapshot.createdUrls.length;
  const partialCard = await clickWithin('partial-image', '[data-testid="io-artifact-read-button"]');
  await waitForText(partialCard, 'complete');
  assert.equal(await partialCard.$('[data-testid="io-artifact-image-preview"]').isExisting(), false);

  const streamCard = await clickWithin('partial-stream-image', 'button[aria-label^="Read artifact stream"]');
  await waitForText(streamCard, 'complete');
  assert.equal(await streamCard.$('[data-testid="io-artifact-image-preview"]').isExisting(), false);
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.equal(snapshot.createdUrls.length, urlsBeforePartial, 'incomplete image bodies must be rejected before URL creation');
  assert.deepEqual(snapshot.streamRequests, [{
    artifact_id: 'payload-partial-stream-image',
    byte_range_start: 0,
    byte_range_end_exclusive: 65_536,
  }]);

  console.log('checking read failure and bounded text');
  const failedCard = await clickWithin('failed-image', '[data-testid="io-artifact-read-button"]');
  await waitForText(failedCard, 'fixture retained body read failed');

  const textCard = await clickWithin('bounded-text', '[data-testid="io-artifact-read-button"]');
  await waitForText(textCard, 'text truncated');
  const textLength = await browser.execute((selector) => document.querySelector(selector)?.textContent?.length ?? -1, `${cardSelector('bounded-text')} pre`);
  assert.equal(textLength, 32_000);
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.deepEqual(snapshot.readRequests.find((request) => request.artifact_id === 'payload-bounded-text'), {
    artifact_id: 'payload-bounded-text',
    byte_range_start: 0,
    byte_range_end_exclusive: 65_536,
  });

  console.log('checking stale pending read');
  await clickWithin('stale-image', '[data-testid="io-artifact-read-button"]');
  await browser.execute(() => window.d02.removeArtifact('stale-image'));
  const refresh = await browser.$('[data-testid="io-inspector-page"] button:nth-of-type(2)');
  await refresh.click();
  await browser.waitUntil(
    async () => browser.execute((selector) => !document.querySelector(selector), cardSelector('stale-image')),
    { timeout: 10_000, timeoutMsg: 'stale artifact was not removed after refresh' },
  );
  const urlsBeforeStaleResolution = (await browser.execute(() => window.d02.snapshot())).createdUrls.length;
  await browser.execute(() => window.d02.resolveStaleRead());
  await delay(100);
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.equal(snapshot.artifacts.includes('stale-image'), false);
  assert.equal(snapshot.createdUrls.length, urlsBeforeStaleResolution, 'stale read must not create a preview URL');

  console.log('checking preview cleanup on inspection failure');
  await clickWithin('valid-image', '[data-testid="io-artifact-read-button"]');
  await browser.waitUntil(async () => (await browser.execute(() => window.d02.snapshot())).createdUrls.length
    === urlsBeforeStaleResolution + 1);
  snapshot = await browser.execute(() => window.d02.snapshot());
  const urlBeforeInspectionFailure = snapshot.createdUrls.at(-1);
  await browser.execute(() => window.d02.rejectInspectionOnce());
  await refresh.click();
  await browser.waitUntil(
    async () => browser.execute((selector) => !document.querySelector(selector), cardSelector('valid-image')),
    { timeout: 10_000, timeoutMsg: 'artifact cards remained after inspection refresh failure' },
  );
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.ok(snapshot.revokedUrls.includes(urlBeforeInspectionFailure),
    'inspection failure must revoke the prior preview URL before clearing cards');
  await refresh.click();
  await card('unmount-pending-image');

  console.log('checking pending read completion after unmount');
  await clickWithin('unmount-pending-image', '[data-testid="io-artifact-read-button"]');
  await browser.waitUntil(async () => (await browser.execute(() => window.d02.snapshot())).readRequests
    .some((request) => request.artifact_id === 'payload-unmount-pending-image'));
  snapshot = await browser.execute(() => window.d02.snapshot());
  const liveUrls = snapshot.createdUrls.filter((url) => !snapshot.revokedUrls.includes(url));
  const urlsBeforeUnmountCompletion = snapshot.createdUrls.length;
  await browser.executeAsync((done) => window.d02.unmount().then(done));
  await browser.execute(() => window.d02.resolveUnmountRead());
  await delay(100);
  snapshot = await browser.execute(() => window.d02.snapshot());
  assert.equal(snapshot.createdUrls.length, urlsBeforeUnmountCompletion,
    'a read completing after unmount must not create a preview URL');
  for (const url of liveUrls) assert.ok(snapshot.revokedUrls.includes(url), `unmount did not revoke ${url}`);

  console.log(`PASS real WebKitGTK: PNG ${await browser.execute(() => window.d02.pngByteLength)} bytes decoded 320x320; full-read, malformed/partial/failure, text bound, stale read, and URL lifecycle checks passed`);
} catch (error) {
  if (driver) console.error(driverOutput);
  throw error;
} finally {
  console.log('cleaning up WebKit driver');
  if (browser) {
    await Promise.race([browser.deleteSession().catch(() => {}), delay(2_000)]);
  }
  if (driver && driver.exitCode === null && driver.signalCode === null) {
    const exited = new Promise((resolve) => driver.once('exit', () => resolve(true)));
    process.kill(-driver.pid, 'SIGTERM');
    const exitedAfterTerm = await Promise.race([exited, delay(3_000).then(() => false)]);
    if (!exitedAfterTerm && driver.exitCode === null && driver.signalCode === null) {
      const forcedExit = new Promise((resolve) => driver.once('exit', resolve));
      process.kill(-driver.pid, 'SIGKILL');
      await forcedExit;
    }
  }
  console.log('closing Vite server');
  if (server) await server.close();
  console.log('cleanup complete');
}
