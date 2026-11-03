  catalog() {
    return [
      { id: 'run_01JB2K7Q3M', wf: 'ingest-docs-rag', ver: '1.4.0', status: 'running', runtime: 'llama.cpp', device: 'cuda:0', deviceName: 'RTX 4090', vram: 17.2, queue: 0, prio: 0, est: 'About 2 min left', node: 'embed_chunks', pct: '61%', reason: '', started: '20:01:40', dur: '4m 12s', updated: '20:05:51', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2K9F0A', wf: 'image-gen-sdxl', ver: '0.9.2', status: 'running', runtime: 'diffusers', device: 'cuda:1', deviceName: 'RTX 3090', vram: 9.8, queue: 0, prio: 0, est: 'About 40 s left', node: 'sample', pct: 'step 18 of 30', reason: '', started: '20:04:02', dur: '1m 49s', updated: '20:05:50', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2KB4ZT', wf: 'transcribe-meeting', ver: '2.1.0', status: 'waiting', runtime: 'whisper', device: 'cuda:0', deviceName: 'RTX 4090', vram: 8, queue: 1, prio: 5, est: 'Starts after ingest-docs-rag', node: '', pct: '', reason: 'needs 8 GB on cuda:0, 6.8 GB free', started: '—', dur: '—', updated: '20:05:12', client: 'cli', session: 's-9c01' },
      { id: 'run_01JB2KC1PX', wf: 'summarize-daily', ver: '1.0.3', status: 'queued', runtime: 'llama.cpp', device: 'cuda:1', deviceName: 'RTX 3090', vram: 6, queue: 2, prio: 0, est: 'About 6 min', node: '', pct: '', reason: 'next on cuda:1, model cached', started: '—', dur: '—', updated: '20:04:40', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2KD8RN', wf: 'embed-corpus', ver: '1.4.0', status: 'queued', runtime: 'llama.cpp', device: '', deviceName: '', vram: 7, queue: 3, prio: 0, est: 'About 9 min', node: '', pct: '', reason: 'cold load, 12 s', started: '—', dur: '—', updated: '20:04:41', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2KE2QW', wf: 'image-gen-sdxl', ver: '0.9.2', status: 'queued', runtime: 'diffusers', device: '', deviceName: '', vram: 9.8, queue: 4, prio: 0, est: 'About 2 min', node: '', pct: '', reason: 'model cached', started: '—', dur: '—', updated: '20:04:55', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2JZ0MT', wf: 'summarize-daily', ver: '1.0.3', status: 'succeeded', runtime: 'llama.cpp', device: 'cuda:0', deviceName: 'RTX 4090', vram: 6, queue: 0, prio: 0, est: 'Finished in 38 s', node: '', pct: '', reason: '', started: '19:52:10', dur: '38 s', updated: '19:52:48', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2JV7KD', wf: 'embed-corpus', ver: '1.3.9', status: 'succeeded', runtime: 'llama.cpp', device: 'cuda:0', deviceName: 'RTX 4090', vram: 7, queue: 0, prio: 0, est: 'Finished in 11m 03s', node: '', pct: '', reason: '', started: '19:38:01', dur: '11m 03s', updated: '19:49:04', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2JT3HB', wf: 'image-gen-sdxl', ver: '0.9.1', status: 'failed', runtime: 'diffusers', device: 'cuda:1', deviceName: 'RTX 3090', vram: 14.2, queue: 0, prio: 0, est: 'Stopped after 22 s', node: 'load_model', pct: '', reason: 'out of memory, needed 14.2 GB', started: '19:35:20', dur: '22 s', updated: '19:35:42', client: 'gui', session: 's-4f2a' },
      { id: 'run_01JB2JQ9XA', wf: 'transcribe-meeting', ver: '2.0.0', status: 'cancelled', runtime: 'whisper', device: 'cuda:0', deviceName: 'RTX 4090', vram: 8, queue: 0, prio: 0, est: 'Stopped after 1m 10s', node: 'transcribe', pct: '', reason: 'cancelled from the GUI', started: '19:30:05', dur: '1m 10s', updated: '19:31:15', client: 'gui', session: 's-4f2a' }
    ];
  }
  wfMap() {
    return {
      'ingest-docs-rag': { model: 'pumas://models/bge-m3', nodes: [['read_docs', 'read-file', 'text'], ['chunking', 'expand-settings', 'settings'], ['split', 'processing', 'chunks'], ['embed_chunks', 'embedding', 'embedding'], ['index', 'vector-output', ''], ['manifest', 'write-file', '']] },
      'image-gen-sdxl': { model: 'pumas://models/sdxl-base-1.0', nodes: [['prompt', 'text-input', 'text'], ['load_model', 'puma-lib', 'model_path'], ['sample', 'pytorch-inference', 'latents'], ['decode', 'image-output', 'image'], ['unload', 'unload-model', '']] },
      'transcribe-meeting': { model: 'pumas://models/whisper-large-v3', nodes: [['audio', 'audio-input', 'audio'], ['transcribe', 'onnx-inference', 'transcript'], ['notes', 'text-output', 'text']] },
      'summarize-daily': { model: 'pumas://models/qwen3-8b-instruct', nodes: [['fetch', 'read-file', 'text'], ['summarize', 'llm-inference', 'response'], ['write', 'write-file', '']] },
      'embed-corpus': { model: 'pumas://models/bge-m3', nodes: [['read', 'read-file', 'text'], ['embed', 'embedding', 'embedding'], ['index', 'vector-output', '']] }
    };
  }
  nodeStates(run) {
    const wf = this.wfMap()[run.wf] || { nodes: [['start', 'processing', 'out'], ['end', 'text-output', '']] };
    const idx = wf.nodes.findIndex((n) => n[0] === run.node);
    return wf.nodes.map(([id, type, port], i) => {
      let state = 'queued';
      if (run.status === 'succeeded') state = 'succeeded';
      else if (run.status === 'running') state = i < idx ? 'succeeded' : i === idx ? 'running' : 'queued';
      else if (run.status === 'failed') state = i < idx ? 'succeeded' : i === idx ? 'failed' : 'skipped';
      else if (run.status === 'cancelled') state = i < idx ? 'succeeded' : i === idx ? 'cancelled' : 'skipped';
      const glyph = state === 'skipped' ? 'g-cancelled' : 'g-' + state;
      return { id, type, port, state, glyph, label: state === 'skipped' ? 'skipped' : state };
    });
  }
  decorateRun(r) {
    const labels = { running: 'Running', waiting: 'Waiting', queued: 'Queued', succeeded: 'Succeeded', failed: 'Failed', cancelled: 'Cancelled' };
    const isLive = r.status === 'running' || r.status === 'waiting' || r.status === 'queued';
    let context = '';
    if (r.status === 'running') context = ', ' + r.node + (r.pct ? ' ' + r.pct : '');
    else if (r.status === 'waiting') context = ', ' + r.reason;
    else if (r.status === 'queued') context = ', #' + r.queue + ' in queue';
    else if (r.reason) context = ', ' + r.reason;
    else context = ', ' + r.dur;
    const placement = r.device ? r.runtime + ' on ' + r.device : r.runtime + ', device not assigned yet';
    const placementLong = r.device ? r.runtime + ' on ' + r.device + ' (' + r.deviceName + ')' : 'Not assigned to a device yet';
    let progress = r.est;
    if (r.status === 'running') progress = r.node + ', ' + r.pct;
    else if (r.status === 'waiting') progress = 'Waiting: ' + r.reason;
    else if (r.status === 'queued') progress = 'Queue position ' + r.queue + ', ' + r.reason;
    else if (r.status === 'failed') progress = 'Failed in ' + r.node + ': ' + r.reason;
    const chips = [];
    if (r.device) chips.push({ text: r.runtime + ' on ' + r.device });
    if (r.prio) chips.push({ text: 'priority ' + r.prio });
    if (r.queue) chips.push({ text: '#' + r.queue + ' in queue' });
    return { ...r, glyph: 'g-' + r.status, stClass: r.status === 'failed' ? 'st-failed' : '', statusLabel: labels[r.status], contextSuffix: context, placement, placementLong, progress, chips, isLive,
      model: (this.wfMap()[r.wf] || {}).model || 'none', hoverOn: () => { this._hoverRun = r.id; }, hoverOff: () => { if (this._hoverRun === r.id) this._hoverRun = null; } };
  }
  ctxBind() {
    if (this._ctxBound || typeof document === 'undefined') return;
    this._ctxBound = true;
    const isField = (t) => t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT');
    document.addEventListener('keydown', (e) => {
      const k = (e.key || '').toLowerCase(); const mod = e.ctrlKey || e.metaKey;
      if (e.key === 'Escape' && this.state && this.state.pickerOpen) { this.setState({ pickerOpen: false }); return; }
      if (!mod || isField(e.target)) return;
      if (k === 'c' && this._hoverRun) this.copyRun(this._hoverRun);
      if (k === 'v' && this._hoverCtx) { e.preventDefault(); this.pasteRun(); }
    });
    document.addEventListener('mousedown', (e) => {
      if (this.state && this.state.pickerOpen && !(e.target.closest && e.target.closest('.ctxwrap'))) this.setState({ pickerOpen: false });
    });
    window.addEventListener('storage', (e) => { if (e.key === 'pg.run') this.setState({ run: e.newValue || null }); });
  }
  copyRun(id) {
    try { localStorage.setItem('pg.clip', id); } catch (_) {}
    if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(id).catch(() => {});
    this.setState({ copied: id });
    clearTimeout(this._copyT); this._copyT = setTimeout(() => this.setState({ copied: null }), 1600);
  }
  pasteRun() {
    const apply = (text) => { const id = (text || '').trim(); if (this.catalog().some((r) => r.id === id)) this.pickRun(id); else this.setState({ pickerOpen: true, pickerQuery: id }); };
    let fallback = ''; try { fallback = localStorage.getItem('pg.clip') || ''; } catch (_) {}
    if (navigator.clipboard && navigator.clipboard.readText) navigator.clipboard.readText().then(apply, () => apply(fallback)); else apply(fallback);
  }
  pickRun(id) {
    try { localStorage.setItem('pg.run', id || ''); } catch (_) {}
    this.setState({ run: id || null, pickerOpen: false, pickerQuery: '', confirmCancel: false });
  }
  ctxRunId(fallback) {
    const st = this.state || {};
    if ('run' in st) return st.run;
    try { const v = localStorage.getItem('pg.run'); if (v !== null) return v || null; } catch (_) {}
    return fallback;
  }
  ctxVals(fallbackId) {
    this.ctxBind();
    const st = this.state || {};
    const runId = this.ctxRunId(fallbackId);
    const all = this.catalog();
    const selRaw = all.find((r) => r.id === runId);
    const sel = this.decorateRun(selRaw || all[0]);
    const qRaw = st.pickerQuery || ''; const q = qRaw.toLowerCase();
    const match = (r) => !q || r.wf.includes(q) || r.id.toLowerCase().includes(q) || ('v' + r.ver).includes(q);
    const groups = [['Running', ['running']], ['Waiting for memory', ['waiting']], ['Queued', ['queued']], ['Finished today', ['succeeded', 'failed', 'cancelled']]]
      .map(([title, keys]) => { const runs = all.filter((r) => keys.includes(r.status) && match(r)).map((r) => ({ ...this.decorateRun(r), selected: r.id === runId, pick: () => this.pickRun(r.id) })); return { title, n: runs.length, runs }; })
      .filter((g) => g.n);
    const total = groups.reduce((n, g) => n + g.n, 0);
    return {
      runId, hasSel: !!selRaw, noSel: !selRaw, sel, ctxHas: !!selRaw, ctxNone: !selRaw, ctxRun: sel,
      pickerOpen: !!st.pickerOpen, pickerQuery: qRaw, pickerGroups: groups,
      pickerEmpty: total === 0, pickerEmptyText: q.startsWith('run_') ? 'No run with id ' + qRaw + ' in today\'s history.' : 'No runs match.',
      togglePicker: () => this.setState({ pickerOpen: !st.pickerOpen }),
      openPicker: () => this.setState({ pickerOpen: true }),
      setQuery: (e) => { const v = e.target.value; if (all.some((r) => r.id === v.trim())) this.pickRun(v.trim()); else this.setState({ pickerQuery: v }); },
      clearSel: () => this.pickRun(null),
      ctxHoverOn: () => { this._hoverCtx = true; }, ctxHoverOff: () => { this._hoverCtx = false; },
      copied: !!st.copied, copiedId: st.copied || ''
    };
  }
