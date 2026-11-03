// Network page (M4) stand-in. Inject after fixture.js: it keeps the base host's
// run list (workflow_run_list_query is delegated to it) and answers the local
// network status, scheduler timeline, library usage and node status queries
// the Network page calls, plus the event listen/unlisten the projection
// subscription needs. Every other command falls through to the base host.
(() => {
  const base = window.__TAURI_INTERNALS__;
  if (!base) throw new Error('fixture-network.js needs fixture.js injected first');
  const baseInvoke = base.invoke;
  const now = Date.now();
  const GIB = 1073741824;
  const projection = (name, seq) => ({ projection_name: name, projection_version: 1, last_applied_event_seq: seq, status: 'current', rebuilt_at_ms: null, updated_at_ms: now });

  // Placements follow the base fixture's running and queued runs.
  const placementFor = (run) => ({
    workflow_run_id: run.workflow_run_id,
    workflow_execution_session_id: 'wes_' + run.workflow_run_id.slice(4, 12),
    workflow_id: run.workflow_id,
    state: run.status === 'running' ? 'running' : 'queued',
    runtime_loaded: run.status === 'running',
    model_cache_state: run.status === 'running' ? 'cache_hit' : 'unknown',
    required_backends: [run.selected_runtime_id || 'llama.cpp'],
    required_models: run.workflow_id === 'image-gen-sdxl' ? ['pumas://models/sdxl-base-1.0'] : ['pumas://models/bge-m3'],
  });

  const networkStatus = (runs) => {
    const live = runs.filter((r) => ['running', 'queued', 'delayed'].includes(r.status));
    const active = live.filter((r) => r.status === 'running');
    const queued = live.filter((r) => r.status !== 'running');
    return {
      local_node: {
        node_id: 'local',
        display_name: 'orangecream',
        captured_at_ms: now - 4000,
        transport_state: 'connected',
        system: {
          hostname: 'orangecream',
          os_name: 'Linux',
          os_version: '7.0',
          kernel_version: '7.0.0-31-generic',
          cpu: { logical_core_count: 16, average_usage_percent: 12.4 },
          memory: { total_bytes: 64 * GIB, used_bytes: 21.4 * GIB, available_bytes: 42.6 * GIB },
          disks: [
            { name: 'nvme0n1p2', mount_point: '/', total_bytes: 1024 * GIB, available_bytes: 212 * GIB },
            { name: 'sda1', mount_point: '/media/OrangeCream', total_bytes: 4096 * GIB, available_bytes: 1433 * GIB },
          ],
          network_interfaces: [
            { name: 'enp5s0', total_received_bytes: 41.2 * GIB, total_transmitted_bytes: 3.9 * GIB },
            { name: 'wg0', total_received_bytes: 2.1 * 1048576, total_transmitted_bytes: 640 * 1024 },
          ],
          gpu: { available: false, reason: 'GPU metrics are not collected by the local system metrics provider' },
        },
        scheduler_load: {
          max_sessions: 4,
          active_session_count: 2,
          max_loaded_sessions: 4,
          loaded_session_count: 1,
          active_run_count: active.length,
          queued_run_count: queued.length,
          active_workflow_run_ids: active.map((r) => r.workflow_run_id),
          queued_workflow_run_ids: queued.map((r) => r.workflow_run_id),
          run_placements: live.map(placementFor),
        },
        degradation_warnings: [
          'GPU metrics are not collected by the local system metrics provider',
          'CPU usage averaged over a short sample',
        ],
      },
      peer_nodes: [
        { node_id: 'node-studio', display_name: 'Studio', transport_state: 'connected', last_seen_at_ms: now - 3000 },
        { node_id: 'node-laptop', display_name: 'Laptop', transport_state: 'degraded', last_seen_at_ms: now - 17 * 60000 },
        { node_id: 'node-render', display_name: 'Render box', transport_state: 'pairing_required', last_seen_at_ms: null },
      ],
    };
  };

  const timeline = (runId, workflowId) => {
    const ev = (seq, kind, source, offsetMs, summary, extra) => Object.assign({
      event_seq: seq, event_id: 'evt_' + seq, event_kind: kind, source_component: source,
      occurred_at_ms: now - offsetMs, recorded_at_ms: now - offsetMs, workflow_run_id: runId, workflow_id: workflowId,
      summary, payload_json: '{}',
    }, extra || {});
    return [
      ev(1, 'scheduler_queue_placement', 'scheduler', 252000, 'Placed in the local queue at position 1'),
      ev(2, 'scheduler_run_admitted', 'scheduler', 251400, 'Admitted on this node'),
      ev(3, 'scheduler_model_lifecycle_changed', 'scheduler', 250800, 'Model cache hit for pumas://models/bge-m3'),
      ev(4, 'run_started', 'workflow_service', 250100, 'Run started'),
      ev(5, 'library_asset_accessed', 'library', 249000, 'Read pumas://models/bge-m3'),
      ev(6, 'diagnostic_error_occurred', 'runtime', 61000, 'Slow batch in embed_chunks', { detail: 'batch took 4.2 s, expected under 2 s', error_severity: 'warning' }),
      ev(7, 'node_execution_status', 'node_execution', 2000, 'embed_chunks running'),
    ];
  };

  const nodeStatus = (runId, workflowId) => {
    const node = (id, type, status, runtime, model) => ({
      workflow_run_id: runId, workflow_id: workflowId, workflow_version_id: 'wfv_1', workflow_semantic_version: '1.4.0',
      node_id: id, node_type: type, node_version: '1.0.0', runtime_id: runtime, runtime_version: runtime ? '0.3.2' : null,
      task_id: null, selected_backend_key: null, model_id: model, model_version: model ? '1.0' : null, status,
      started_at_ms: now - 250000, completed_at_ms: null, duration_ms: null, error: null, last_event_seq: 7, last_updated_at_ms: now - 2000,
    });
    return [
      node('read_docs', 'read-file', 'completed', null, null),
      node('split', 'processing', 'completed', null, null),
      node('embed_chunks', 'embedding', 'running', 'llama.cpp', 'pumas://models/bge-m3'),
      node('index', 'vector-output', 'queued', null, null),
    ];
  };

  const assets = (runId) => [
    { asset_id: 'pumas://models/bge-m3', total_access_count: 9, run_access_count: 1, total_network_bytes: 0, last_accessed_at_ms: now - 249000, last_operation: 'read', last_cache_status: 'cache_hit', last_workflow_run_id: runId, last_event_seq: 5, last_updated_at_ms: now - 249000 },
    { asset_id: 'runtime:llama.cpp', total_access_count: 14, run_access_count: 1, total_network_bytes: 2.1 * 1048576, last_accessed_at_ms: now - 250000, last_operation: 'read', last_cache_status: 'cache_hit', last_workflow_run_id: runId, last_event_seq: 4, last_updated_at_ms: now - 250000 },
  ];

  const runList = () => baseInvoke('workflow_run_list_query', {}).then((r) => r.runs);
  const workflowOf = (runs, runId) => (runs.find((r) => r.workflow_run_id === runId) || {}).workflow_id || 'unknown';

  window.__TAURI_INTERNALS__ = Object.assign({}, base, {
    invoke: (cmd, args) => {
      const request = (args && args.request) || {};
      switch (cmd) {
        case 'plugin:event|listen':
          return Promise.resolve(1);
        case 'plugin:event|unlisten':
          return Promise.resolve();
        case 'workflow_local_network_status_query':
          return runList().then((runs) => networkStatus(runs));
        case 'workflow_scheduler_timeline_query':
          return runList().then((runs) => ({ events: timeline(request.workflow_run_id, workflowOf(runs, request.workflow_run_id)), projection_state: projection('scheduler_timeline', 7) }));
        case 'workflow_node_status_query':
          return runList().then((runs) => ({ nodes: nodeStatus(request.workflow_run_id, workflowOf(runs, request.workflow_run_id)), projection_state: projection('node_status', 7) }));
        case 'workflow_library_usage_query':
          return Promise.resolve({ assets: assets(request.workflow_run_id), projection_state: projection('library_usage', 5) });
        default:
          return baseInvoke(cmd, args);
      }
    },
  });
})();
