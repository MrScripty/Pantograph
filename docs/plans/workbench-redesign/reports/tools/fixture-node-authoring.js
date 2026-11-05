// Node Editor (M6b) stand-in for the node authoring commands PR #95's
// NodeLabService calls. Load after fixture.js. The catalog comes from the
// page URL: ?fixtureCatalog=one (default, the nodelab_host example's single
// node), many (seven nodes, so the search field shows), empty, or error.
// ?fixtureClose=fail makes remove_execution fail, so replacing a draft leaves
// a session for the page's cleanup retry.
// Draft graphs follow create_custom_node_draft in
// crates/pantograph-workflow-service/src/workflow/custom_node_authoring_api.rs;
// digests and the blocking diagnostic text are fixture values, not the Rust
// service's.
(() => {
  const base = window.__TAURI_INTERNALS__;
  if (!base) throw new Error('fixture-node-authoring.js needs fixture.js loaded first');
  const params = new URLSearchParams(location.search);
  const mode = params.get('fixtureCatalog') || 'one';
  const closeFails = params.get('fixtureClose') === 'fail';
  const hex = (seed) => {
    let state = 2166136261;
    for (const char of seed) state = Math.imul(state ^ char.charCodeAt(0), 16777619) >>> 0;
    let out = '';
    while (out.length < 64) { state = Math.imul(state ^ (state >>> 15), 2246822519) >>> 0; out += state.toString(16).padStart(8, '0'); }
    return out.slice(0, 64);
  };
  const node = (type, label, description, inputs, outputs) => ({
    identity: { node_type: type, contract_version: '1.0.0', contract_digest: 'contract-blake3:' + hex(type + ':contract'), implementation_digest: 'blake3:' + hex(type + ':impl'), environment_digest: 'blake3:' + hex(type + ':env'), adapter_version: 1 },
    label, description, trust: 'trusted_in_process',
    inputs: inputs.map(([id, portLabel]) => ({ id, label: portLabel })),
    outputs: outputs.map(([id, portLabel]) => ({ id, label: portLabel })),
  });
  const uppercase = node('host-uppercase', 'Host uppercase', 'Pure string transform compiled into this isolated Rust host', [['input', 'Input']], [['output', 'Output']]);
  const many = [
    uppercase,
    node('host-slugify', 'Slugify title', 'Lower-cases a title and joins words with hyphens for file names', [['title', 'Title']], [['slug', 'Slug']]),
    node('host-template', 'Fill template', 'Replaces {name} placeholders in a template with the given values', [['template', 'Template'], ['values', 'Values']], [['text', 'Text']]),
    node('host-redact', 'Redact emails', 'Masks e-mail addresses before text leaves the machine', [['text', 'Text']], [['text', 'Redacted'], ['count', 'Matches']]),
    node('host-trim', 'Trim whitespace', 'Removes leading and trailing whitespace from every line', [['text', 'Text']], [['text', 'Trimmed']]),
    node('host-word-count', 'Word count', 'Counts words and lines in a text', [['text', 'Text']], [['words', 'Words'], ['lines', 'Lines']]),
    node('host-join-lines', 'Join lines', 'Joins lines with a separator', [['text', 'Text'], ['separator', 'Separator']], [['text', 'Joined']]),
  ];
  const catalog = mode === 'empty' ? [] : mode === 'many' ? many : [uppercase];
  const reject = (code, message) => Promise.reject(JSON.stringify({ code, message }));
  const sessions = new Map();
  const saved = new Map();
  let next = 1;
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const diagnostics = (graph) => graph.nodes
    .filter((item) => item.data && item.data.custom_node_identity && !catalog.some((entry) => entry.identity.node_type === item.node_type))
    .map((item) => ({ message: `Node '${item.id}' pins custom node '${item.node_type}', which this host has not admitted`, blocking_submission: true }));
  const open = (graph) => {
    const id = 'edit-' + next++;
    sessions.set(id, clone(graph));
    return id;
  };
  const draft = (id) => {
    const graph = sessions.get(id);
    return { session_id: id, graph_revision: 'rev-' + hex(JSON.stringify(graph)).slice(0, 12), graph: clone(graph), graph_diagnostics: diagnostics(graph) };
  };
  const answers = {
    get_custom_node_authoring_catalog: () => (mode === 'error' ? reject('runtime_not_ready', 'The workflow host did not answer the catalog request.') : { contract_version: 1, nodes: clone(catalog) }),
    create_custom_node_draft: ({ request }) => {
      const entry = catalog.find((item) => item.identity.node_type === request.identity.node_type);
      if (!entry) return reject('invalid_request', 'custom-node implementation is unavailable');
      const graph = { nodes: [{ id: 'custom', node_type: entry.identity.node_type, position: { x: 320, y: 0 }, data: { label: entry.label, custom_node_identity: clone(entry.identity) } }], edges: [] };
      entry.inputs.forEach((port, index) => {
        graph.nodes.push({ id: 'input-' + index, node_type: 'text-input', position: { x: 0, y: index * 160 }, data: { label: port.label, text: '' } });
        graph.edges.push({ id: 'edge-input-' + index, source: 'input-' + index, source_handle: 'text', target: 'custom', target_handle: port.id });
      });
      entry.outputs.forEach((port, index) => {
        graph.nodes.push({ id: 'output-' + index, node_type: 'text-output', position: { x: 640, y: index * 160 }, data: { label: port.label, text: '' } });
        graph.edges.push({ id: 'edge-output-' + index, source: 'custom', source_handle: port.id, target: 'output-' + index, target_handle: 'text' });
      });
      return draft(open(graph));
    },
    get_node_lab_draft: ({ executionId }) => (sessions.has(executionId) ? draft(executionId) : reject('session_not_found', 'No edit session ' + executionId)),
    update_node_data: ({ executionId, nodeId, data }) => {
      const graph = sessions.get(executionId);
      if (!graph) return reject('session_not_found', 'No edit session ' + executionId);
      const target = graph.nodes.find((item) => item.id === nodeId);
      Object.assign(target.data, data);
      return draft(executionId);
    },
    save_workflow: ({ name, graph }) => {
      const path = '.pantograph/workflows/' + name + '.json';
      saved.set(path, clone(graph));
      return path;
    },
    load_workflow: ({ path }) => (saved.has(path) ? { graph: clone(saved.get(path)) } : reject('invalid_request', 'No saved workflow at ' + path)),
    create_workflow_execution_session: ({ graph }) => ({ session_id: open(graph) }),
    remove_execution: ({ executionId }) => closeFails ? reject('runtime_not_ready', 'The workflow host did not confirm closing ' + executionId + '.') : (sessions.delete(executionId) ? null : reject('session_not_found', 'No edit session ' + executionId)),
  };
  const invoke = base.invoke;
  window.__TAURI_INTERNALS__ = Object.assign({}, base, {
    invoke: (cmd, args, options) => (cmd in answers ? Promise.resolve().then(() => answers[cmd](args || {})) : invoke(cmd, args, options)),
  });
})();
