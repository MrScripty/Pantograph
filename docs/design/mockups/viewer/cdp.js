// Drives headless Chrome over the DevTools protocol with Node's built-in WebSocket.
// Usage: node cdp.js steps.json   where steps is [{goto:url}|{eval:js}|{wait:ms}|{shot:file}]
const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const CH = os.homedir() + '/.cache/ms-playwright/chromium_headless_shell-1246/chrome-headless-shell-linux64/chrome-headless-shell';
const PORT = 9333;
const steps = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const chrome = spawn(CH, ['--headless', '--disable-gpu', '--hide-scrollbars=false', '--remote-debugging-port=' + PORT, '--window-size=1440,900', '--user-data-dir=' + fs.mkdtempSync(os.tmpdir() + '/cdp-'), 'about:blank'], { stdio: 'ignore' });
  let targets = null;
  for (let i = 0; i < 50 && !targets; i++) { await sleep(200); try { targets = await (await fetch('http://127.0.0.1:' + PORT + '/json')).json(); } catch (_) {} }
  const page = targets.find((t) => t.type === 'page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0; const pending = new Map();
  ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id && pending.has(d.id)) { pending.get(d.id)(d); pending.delete(d.id); } };
  const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
  await send('Page.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  for (const s of steps) {
    if (s.goto) { await send('Page.navigate', { url: s.goto }); await sleep(s.settle || 1200); }
    else if (s.eval) { const r = await send('Runtime.evaluate', { expression: s.eval, awaitPromise: true, returnByValue: true }); if (r.result && r.result.exceptionDetails) console.log('eval error:', r.result.exceptionDetails.text, s.eval.slice(0, 80)); else if (s.log) console.log(s.log + ':', JSON.stringify(r.result.result.value)); await sleep(s.settle || 250); }
    else if (s.wait) await sleep(s.wait);
    else if (s.shot) { const r = await send('Page.captureScreenshot', { format: 'png' }); fs.writeFileSync(s.shot, Buffer.from(r.result.data, 'base64')); console.log('shot', s.shot); }
  }
  ws.close(); chrome.kill();
})().catch((e) => { console.error(e); process.exit(1); });
