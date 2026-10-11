const tabs = [...document.querySelectorAll('[role="tab"]')];
function activate(tab) {
  for (const item of tabs) {
    const selected = item === tab;
    item.setAttribute('aria-selected', String(selected));
    item.tabIndex = selected ? 0 : -1;
    document.getElementById(item.dataset.panel).hidden = !selected;
  }
}
for (const [index, tab] of tabs.entries()) {
  tab.addEventListener('click', () => activate(tab));
  tab.addEventListener('keydown', event => {
    const next = event.key === 'ArrowRight' ? (index + 1) % tabs.length
      : event.key === 'ArrowLeft' ? (index + tabs.length - 1) % tabs.length
      : event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : null;
    if (next !== null) { event.preventDefault(); activate(tabs[next]); tabs[next].focus(); }
  });
}
let latestRequest = 0;
async function loadRecording() {
  const request = ++latestRequest;
  const name = document.getElementById('recording').value;
  const path = `images/scheduler-demo-${name}.json?v=7fc3584`;
  document.getElementById('download').href = path;
  const list = document.getElementById('event-list');
  list.replaceChildren();
  document.getElementById('event-detail').textContent = '';
  document.getElementById('trace-summary').textContent = 'Loading recorded evidence…';
  try {
    const response = await fetch(path);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const report = await response.json();
    if (request !== latestRequest) return;
    const result = report.execution;
    document.getElementById('trace-summary').textContent = result.ok
      ? `Recorded output: ${result.result.outputs[0].value} · ${report.diagnostics.length} events · ${result.result.workflow_run_id}`
      : `Expected rejection: ${result.error} · ${report.diagnostics.length} events`;
    for (const event of report.diagnostics) {
      const button = document.createElement('button');
      button.textContent = `${event.event_seq.toString().padStart(2, '0')}  ${event.event_kind}${event.node_id ? ' / ' + event.node_id : ''}`;
      button.setAttribute('aria-pressed', 'false');
      button.onclick = () => {
        for (const other of list.children) other.setAttribute('aria-pressed', 'false');
        button.setAttribute('aria-pressed', 'true');
        document.getElementById('event-detail').textContent = JSON.stringify({...event, payload_json: JSON.parse(event.payload_json)}, null, 2);
      };
      list.append(button);
    }
    list.firstElementChild?.click();
  } catch (error) {
    if (request === latestRequest) document.getElementById('trace-summary').textContent = `Recording unavailable: ${error.message}`;
  }
}
document.getElementById('recording').addEventListener('change', loadRecording);
loadRecording();
