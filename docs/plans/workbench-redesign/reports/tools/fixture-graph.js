// Graph page (M5) stand-in host. Inject after fixture.js: it keeps the base
// fixture's run list and answers what the Graph page calls in edit mode
// (node definitions, workflow list/load, edit session) and in run mode
// (run graph projection, node status projection). Everything else falls
// through to the base fixture, which rejects it like an unreachable host.
(() => {
  const base = window.__TAURI_INTERNALS__.invoke;
  const now = Date.now();
  try {
    localStorage.setItem('pantograph.lastGraph', JSON.stringify({ id: 'ingest-docs-rag', type: 'workflow' }));
    localStorage.setItem('pantograph.workflowSemanticVersion.ingest-docs-rag', '1.4.1');
  } catch (_) {}

  const port = (id, label, data_type) => ({ id, label, data_type, required: false, multiple: false });
  const def = (node_type, category, label, inputs, outputs, description) => ({
    node_type, category, label, description: description || label, io_binding_origin: 'integrated',
    inputs, outputs, execution_mode: 'reactive',
  });
  const defs = [
    def('text-input', 'input', 'Text input', [], [port('text', 'Text', 'string')]),
    def('number-input', 'input', 'Number input', [], [port('value', 'Value', 'number')]),
    def('selection-input', 'input', 'Selection', [], [port('value', 'Value', 'string')]),
    def('read-file', 'input', 'Read file', [], [port('text', 'Text', 'string')], 'Reads files from a folder'),
    def('audio-input', 'input', 'Audio input', [], [port('audio', 'Audio', 'audio')]),
    def('llm-inference', 'processing', 'LLM inference', [port('prompt', 'Prompt', 'prompt'), port('system', 'System', 'string'), port('model', 'Model', 'string'), port('tools', 'Tools', 'tools')], [port('response', 'Response', 'string'), port('stream', 'Stream', 'stream'), port('kv', 'KV cache', 'kv_cache')]),
    def('rag-search', 'processing', 'Embed chunks', [port('chunks', 'Chunks', 'json'), port('model', 'Model', 'string')], [port('embedding', 'Embedding', 'embedding'), port('metadata', 'Metadata', 'json')]),
    def('tool-loop', 'control', 'Split text', [port('text', 'Text', 'string'), port('size', 'Chunk size', 'number')], [port('chunks', 'Chunks', 'json')]),
    def('puma-lib', 'tool', 'Puma lib', [], [port('model_path', 'Model path', 'string'), port('model_id', 'Model id', 'string')]),
    def('unload-model', 'tool', 'Unload model', [port('model', 'Model', 'string')], []),
    def('text-output', 'output', 'Text output', [port('text', 'Text', 'string')], []),
    def('image-output', 'output', 'Image output', [port('image', 'Image', 'image')], []),
    def('vector-output', 'output', 'Vector output', [port('embedding', 'Embedding', 'embedding')], []),
    def('write-file', 'output', 'Write file', [port('metadata', 'Metadata', 'json'), port('chunks', 'Chunks', 'json')], []),
  ];
  const byType = Object.fromEntries(defs.map((d) => [d.node_type, d]));
  const node = (id, type, x, y, label, extra) => ({ id, node_type: type, position: { x, y }, data: Object.assign({ label, definition: byType[type] }, extra || {}) });
  const edge = (source, sh, target, th) => ({ id: `${source}-${sh}-${target}-${th}`, source, source_handle: sh, target, target_handle: th });

  const graphs = {
    'ingest-docs-rag': {
      nodes: [
        node('read_docs', 'read-file', 40, 60, 'Source folder', { path: '~/docs/handbook', glob: '**/*.md' }),
        node('chunk_size', 'number-input', 40, 300, 'Chunk size', { value: 512 }),
        node('split', 'tool-loop', 330, 140, 'Split', { strategy: 'markdown headings', overlap: 64 }),
        node('embed_chunks', 'rag-search', 620, 170, 'embed_chunks', { model: 'pumas://models/bge-m3', batch_size: 64, device: 'auto' }),
        node('index', 'vector-output', 920, 110, 'Index', { collection: 'handbook' }),
        node('manifest', 'write-file', 920, 320, 'Manifest', { path: '~/docs/handbook.index.json' }),
      ],
      edges: [
        edge('read_docs', 'text', 'split', 'text'),
        edge('chunk_size', 'value', 'split', 'size'),
        edge('split', 'chunks', 'embed_chunks', 'chunks'),
        edge('embed_chunks', 'embedding', 'index', 'embedding'),
        edge('embed_chunks', 'metadata', 'manifest', 'metadata'),
        edge('split', 'chunks', 'manifest', 'chunks'),
      ],
    },
    'image-gen-sdxl': {
      nodes: [
        node('prompt', 'text-input', 40, 120, 'prompt', { text: 'A pantograph on a drafting table' }),
        node('load_model', 'puma-lib', 40, 300, 'load_model', { model: 'pumas://models/sdxl-base-1.0' }),
        node('sample', 'llm-inference', 340, 160, 'sample', { steps: 30, cfg: 6.5 }),
        node('decode', 'image-output', 660, 160, 'decode', { format: 'png' }),
      ],
      edges: [
        edge('prompt', 'text', 'sample', 'prompt'),
        edge('load_model', 'model_path', 'sample', 'model'),
        edge('sample', 'response', 'decode', 'image'),
      ],
    },
  };

  const workflows = [
    ['ingest-docs-rag', 'ingest-docs-rag', now - 3600e3],
    ['image-gen-sdxl', 'image-gen-sdxl', now - 7200e3],
    ['transcribe-meeting', 'transcribe-meeting', now - 5 * 86400e3],
    ['summarize-daily', 'summarize-daily', now - 6 * 86400e3],
  ].map(([id, name, ms]) => ({ id, name, created: new Date(ms).toISOString(), modified: new Date(ms).toISOString() }));

  const runGraphs = {
    'run_7c9e6679-7425-40de-944b-e07fc1f90ae7': ['ingest-docs-rag', '1.4.0'],
    'run_3f2504e0-4f89-11d3-9a0c-0305e82c3301': ['image-gen-sdxl', '0.9.1'],
    'run_16fd2706-8baf-433b-82eb-8c7fada847da': ['summarize-daily', '1.0.3'],
  };
  const nodeStatuses = {
    'run_7c9e6679-7425-40de-944b-e07fc1f90ae7': [
      ['read_docs', 'completed', { duration_ms: 1200 }], ['chunk_size', 'completed', { duration_ms: 3 }],
      ['split', 'completed', { duration_ms: 4100, execution_cache_status: 'fresh_execution' }],
      ['embed_chunks', 'running', { started_at_ms: now - 252000, runtime_id: 'llama.cpp', selected_backend_key: 'cuda:0', model_id: 'pumas://models/bge-m3' }],
    ],
    'run_3f2504e0-4f89-11d3-9a0c-0305e82c3301': [
      ['prompt', 'completed', { duration_ms: 2 }],
      ['load_model', 'failed', { error: 'out of memory, needed 14.2 GB on cuda:1', error_code: 'runtime_oom', error_severity: 'fatal', error_phase: 'model_load', error_event_id: 'evt_1' }],
    ],
  };

  const runGraphProjection = (runId) => {
    const entry = runGraphs[runId];
    if (!entry) return null;
    const [workflowId, version] = entry;
    const graph = graphs[workflowId];
    if (!graph) return null;
    return {
      workflow_run_id: runId, workflow_id: workflowId, workflow_version_id: `wfver-${workflowId}-${version}`,
      workflow_presentation_revision_id: `wfpres-${workflowId}`, workflow_semantic_version: version,
      workflow_execution_fingerprint: 'fp-' + workflowId, snapshot_created_at_ms: now - 300000,
      workflow_version_created_at_ms: now - 86400e3, presentation_revision_created_at_ms: now - 86400e3,
      graph,
      graph_diagnostics: [],
      executable_topology: {
        schema_version: 1,
        nodes: graph.nodes.map((n) => ({ node_id: n.id, node_type: n.node_type, contract_version: '1.0.0', behavior_digest: 'sha256:' + n.id.padEnd(12, '0').slice(0, 12) })),
        edges: graph.edges.map((e) => ({ source_node_id: e.source, source_port_id: e.source_handle, target_node_id: e.target, target_port_id: e.target_handle })),
      },
      presentation_metadata: {
        schema_version: 1,
        nodes: graph.nodes.map((n) => ({ node_id: n.id, position: n.position })),
        edges: graph.edges.map((e) => ({ edge_id: e.id, source_node_id: e.source, source_port_id: e.source_handle, target_node_id: e.target, target_port_id: e.target_handle })),
      },
      graph_settings: {
        schema_version: 1,
        nodes: graph.nodes.map((n) => {
          const { label: _label, definition: _definition, ...settings } = n.data;
          return { node_id: n.id, node_type: n.node_type, data: settings };
        }),
      },
    };
  };

  const handlers = {
    get_node_definitions: () => defs,
    list_workflows: () => workflows,
    load_workflow: (args) => {
      const stem = String(args.path || '').split('/').pop().replace(/\.json$/, '');
      const meta = workflows.find((w) => w.id === stem);
      if (!meta || !graphs[stem]) throw new Error('workflow not found: ' + stem);
      return { version: '1.0', metadata: Object.assign({}, meta), graph: graphs[stem] };
    },
    create_workflow_execution_session: () => ({ session_id: 'fixture-edit-session', session_kind: 'edit' }),
    workflow_run_graph_query: (args) => ({ run_graph: runGraphProjection(args.request.workflow_run_id) }),
    workflow_node_status_query: (args) => {
      const runId = args.request.workflow_run_id;
      const [workflowId] = runGraphs[runId] || ['unknown'];
      const nodes = (nodeStatuses[runId] || []).map(([node_id, status, extra], index) => Object.assign({
        workflow_run_id: runId, workflow_id: workflowId, node_id, status, last_event_seq: index + 1, last_updated_at_ms: now,
      }, extra));
      return { nodes, projection_state: { projection_name: 'node_status', projection_version: 5, last_applied_event_seq: 9, status: 'current', rebuilt_at_ms: null, updated_at_ms: now } };
    },
  };

  window.__TAURI_INTERNALS__.invoke = (cmd, args) => {
    const handler = handlers[cmd];
    if (!handler) return base(cmd, args);
    try {
      return Promise.resolve(handler(args || {}));
    } catch (error) {
      return Promise.reject(error);
    }
  };
})();
