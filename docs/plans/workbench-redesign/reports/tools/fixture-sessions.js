// Sessions (M8) stand-in. Load after fixture.js: it keeps that file's run list
// and adds the saved-workflow and orchestration listings the Sessions tab
// reads. Every other command still goes to the base fixture (and is rejected
// there unless it is the run-list query).
(() => {
  const base = window.__TAURI_INTERNALS__;
  if (!base) throw new Error('fixture-sessions.js needs fixture.js loaded first');
  const day = 86400000;
  const now = Date.now();
  const iso = (ms) => new Date(ms).toISOString();
  const workflows = [
    { id: 'ingest-docs-rag', name: 'ingest-docs-rag', description: 'Chunk and embed the handbook for RAG', created: iso(now - 9 * day), modified: iso(now - 2 * 3600000) },
    { id: 'image-gen-sdxl', name: 'image-gen-sdxl', description: 'SDXL base image generation', created: iso(now - 30 * day), modified: iso(now - 3 * day) },
    { id: 'summarize-daily', name: 'summarize-daily', description: 'Daily notes summary', created: iso(now - 60 * day), modified: iso(now - 12 * day) },
    { id: 'maid', name: 'Maid', created: '2026-01-31T10:00:00Z', modified: '2026-05-01T09:30:00Z' },
  ];
  const orchestrations = [
    { id: 'coding-agent-orch', name: 'Coding Agent Orchestration', description: 'Wraps Coding Agent with a tool loop', nodeCount: 5 },
  ];
  const answers = {
    list_workflows: () => workflows,
    list_orchestrations: () => orchestrations,
  };
  const invoke = base.invoke;
  window.__TAURI_INTERNALS__ = Object.assign({}, base, {
    invoke: (cmd, args, options) => (cmd in answers ? Promise.resolve(answers[cmd](args)) : invoke(cmd, args, options)),
  });
})();
