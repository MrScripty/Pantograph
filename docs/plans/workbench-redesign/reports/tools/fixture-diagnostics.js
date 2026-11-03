/* global window */
// Diagnostics (M2) stand-in host. Inject after fixture.js: it reuses the base
// fixture's runs and answers the projection queries DiagnosticsPage calls
// (run detail, scheduler timeline, node status, I/O retention, run list scoped
// to one workflow) plus the event `listen` call, so live updates subscribe.
// The failed image-gen-sdxl run carries a terminal runtime_model_load_failed
// error with its node error in the timeline. Every other command still goes to
// the base fixture, which rejects it like an unreachable host.
(() => {
  const host = window.__TAURI_INTERNALS__;
  const baseInvoke = host.invoke;
  const now = Date.now();
  const FAILED_RUN = 'run_3f2504e0-4f89-11d3-9a0c-0305e82c3301';
  const state = (name, seq) => ({ projection_name: name, projection_version: 1, last_applied_event_seq: seq, status: 'current', rebuilt_at_ms: null, updated_at_ms: now });
  const baseRuns = () => baseInvoke('workflow_run_list_query', { request: {} }).then((r) => r.runs);

  const nodesByWorkflow = {
    'image-gen-sdxl': ['prompt', 'load_model', 'sample', 'decode', 'unload'],
    'ingest-docs-rag': ['read_docs', 'chunking', 'split', 'embed_chunks', 'index', 'manifest'],
    'transcribe-meeting': ['audio', 'transcribe', 'notes'],
    'summarize-daily': ['fetch', 'summarize', 'write'],
    'embed-corpus': ['read', 'embed', 'index'],
  };

  function times(run) {
    if (run.workflow_run_id === FAILED_RUN) {
      const completed = run.completed_at_ms;
      const started = completed - 22000;
      return { accepted: started - 18100, enqueued: started - 18099, started, completed, duration: 22000 };
    }
    const completed = run.completed_at_ms || null;
    const started = completed ? completed - (run.duration_ms || 30000) : run.status === 'running' ? now - 252000 : null;
    const accepted = (started || run.last_updated_at_ms) - 28000;
    return { accepted, enqueued: accepted + 1, started, completed, duration: run.duration_ms || null };
  }

  function detail(run) {
    const t = times(run);
    const failed = run.workflow_run_id === FAILED_RUN;
    return Object.assign({}, run, {
      accepted_at_ms: t.accepted, enqueued_at_ms: t.enqueued, started_at_ms: t.started, completed_at_ms: t.completed, duration_ms: t.duration,
      scheduler_policy_id: 'fair-share', retention_policy_id: 'metadata-7d', bucket_id: 'default',
      selected_runtime_variant_id: run.selected_runtime_id ? run.selected_runtime_id + '.cuda' : null,
      selected_backend_key: run.selected_runtime_id || null,
      selected_model_id: failed ? 'pumas://models/sdxl-base-1.0' : null,
      selected_device_class: run.selected_device_id ? 'cuda' : null,
      selected_device_id: failed ? 'cuda:1' : run.selected_device_id || null,
      selected_runtime_id: failed ? 'diffusers' : run.selected_runtime_id || null,
      client_id: 'gui', client_session_id: 's-4f2a', workflow_execution_session_id: 'exec-' + run.workflow_run_id.slice(4, 12),
      workflow_run_snapshot_id: 'snap-' + run.workflow_run_id.slice(4, 12), workflow_presentation_revision_id: 'pres-3',
      scheduler_priority: 0, estimate_confidence: 'medium', estimated_queue_wait_ms: 9000, estimated_duration_ms: 31000,
      model_cache_state: failed ? 'cache_miss' : 'cache_hit',
      scheduler_reason: failed ? 'cold model load' : run.scheduler_reason || null,
      latest_error_event_id: failed ? 'evt-runtime-load' : null,
      latest_error_severity: failed ? 'fatal' : null,
      latest_error_phase: failed ? 'runtime_model_load' : null,
      latest_error_code: failed ? 'runtime_model_load_failed' : null,
      latest_error_message: failed ? 'CUDA out of memory, tried to allocate 1.9 GB on cuda:1 with 0.4 GB free' : null,
      fatal_error_event_id: failed ? 'evt-runtime-load' : null,
      error_count: failed ? 1 : 0, warning_count: failed ? 1 : 0,
      terminal_error: failed ? 'runtime model load failed' : null,
      timeline_event_count: failed ? 14 : 6, last_event_seq: 1842,
    });
  }

  function ev(run, seq, at, kind, source, summary, extra) {
    return Object.assign({
      event_seq: seq, event_id: 'evt-' + seq, event_kind: kind, source_component: source, occurred_at_ms: at, recorded_at_ms: at,
      workflow_run_id: run.workflow_run_id, workflow_id: run.workflow_id, summary, detail: null, error_severity: null, error_phase: null,
      related_event_ids: [], payload_json: '{}',
    }, extra || {});
  }

  function timeline(run) {
    const t = times(run);
    if (run.workflow_run_id === FAILED_RUN) {
      const s = t.started;
      return [
        ev(run, 1801, t.accepted, 'run_snapshot_accepted', 'workflow_service', 'accepted from gui, session s-4f2a'),
        ev(run, 1802, t.enqueued, 'scheduler_queue_placement', 'scheduler', 'enqueued fair-share, priority 0, position 2'),
        ev(run, 1806, s - 8805, 'scheduler_estimate_produced', 'scheduler', 'cuda:1 preferred, 9.8 GB free, model not cached', { payload_json: '{"estimate":"cold"}' }),
        ev(run, 1810, s - 2177, 'scheduler_run_admitted', 'scheduler', 'placed on cuda:1 with diffusers 0.31, 14.2 GB requested'),
        ev(run, 1811, s - 2176, 'library_asset_accessed', 'library', 'pumas://models/sdxl-base-1.0, cache miss, 6.9 GB read from disk'),
        ev(run, 1815, s, 'run_started', 'runtime', 'diffusers worker pid 48112 on cuda:1'),
        ev(run, 1816, s + 2, 'node_execution_status', 'node_execution', 'load_model started'),
        ev(run, 1820, s + 4673, 'runtime_capability_observed', 'runtime', 'memory pressure on cuda:1, 22.9 of 24 GB in use', { error_severity: 'warning' }),
        ev(run, 1824, s + 10910, 'scheduler_model_lifecycle_changed', 'runtime', 'unet weights 5.1 GB mapped, vae pending'),
        ev(run, 1830, s + 21753, 'diagnostic_error_occurred', 'scheduler', 'runtime_model_load_failed', { event_id: 'evt-runtime-load', detail: 'CUDA out of memory, tried to allocate 1.9 GB on cuda:1, 0.4 GB free', error_severity: 'fatal', error_phase: 'runtime_model_load' }),
        ev(run, 1831, s + 21797, 'node_execution_status', 'node_execution', 'failed', { event_id: 'evt-node-load', error_severity: 'error', error_phase: 'runtime_model_load' }),
        ev(run, 1833, s + 21803, 'scheduler_reservation_changed', 'runtime', 'worker exited, pid 48112, code 1, 14.2 GB released'),
        ev(run, 1840, t.completed - 6, 'run_terminal', 'scheduler', 'failed after 22 s, 3 downstream nodes not run'),
        ev(run, 1842, t.completed, 'retention_policy_changed', 'retention', 'no payloads retained, metadata kept 7 days'),
      ];
    }
    const s = t.started || t.accepted + 2000;
    const out = [
      ev(run, 101, t.accepted, 'run_snapshot_accepted', 'workflow_service', 'accepted from gui, session s-4f2a'),
      ev(run, 102, t.enqueued, 'scheduler_queue_placement', 'scheduler', run.scheduler_reason || 'enqueued fair-share, priority 0'),
    ];
    if (t.started) {
      out.push(ev(run, 103, s - 1000, 'scheduler_run_admitted', 'scheduler', 'placed on ' + (run.selected_device_id || 'cpu') + ' with ' + (run.selected_runtime_id || 'runtime')));
      out.push(ev(run, 104, s, 'run_started', 'runtime', (run.selected_runtime_id || 'runtime') + ' worker started'));
      out.push(ev(run, 105, s + 1200, 'node_execution_status', 'node_execution', (nodesByWorkflow[run.workflow_id] || ['node'])[0] + ' completed, outputs retained'));
    }
    if (t.completed) out.push(ev(run, 106, t.completed, 'run_terminal', 'scheduler', run.status + ' after ' + Math.round(t.duration / 1000) + ' s'));
    return out;
  }

  function nodes(run) {
    const ids = nodesByWorkflow[run.workflow_id] || ['start', 'end'];
    const failed = run.workflow_run_id === FAILED_RUN;
    return ids.map((id, i) => {
      let status = 'queued';
      if (run.status === 'completed') status = 'completed';
      else if (failed) status = i === 0 ? 'completed' : i === 1 ? 'failed' : 'cancelled';
      else if (run.status === 'running') status = i < 2 ? 'completed' : i === 2 ? 'running' : 'queued';
      return {
        workflow_run_id: run.workflow_run_id, workflow_id: run.workflow_id, workflow_semantic_version: run.workflow_semantic_version,
        node_id: id, node_type: 'node', node_version: '1.0.0', runtime_id: run.selected_runtime_id || (failed ? 'diffusers' : null),
        runtime_version: '0.31', model_id: failed && id === 'load_model' ? 'pumas://models/sdxl-base-1.0' : null, model_version: null,
        status, started_at_ms: null, completed_at_ms: null, duration_ms: null,
        error: failed && i === 1 ? 'runtime model load failed' : null,
        error_event_id: failed && i === 1 ? 'evt-node-load' : null,
        canonical_error_event_id: failed && i === 1 ? 'evt-node-load' : null,
        error_severity: failed && i === 1 ? 'error' : null,
        error_phase: failed && i === 1 ? 'runtime_model_load' : null,
        error_code: failed && i === 1 ? 'runtime_model_load_failed' : null,
        last_event_seq: 1842, last_updated_at_ms: now,
      };
    });
  }

  // Earlier runs of image-gen-sdxl for "Compared with earlier runs": 14 in all with the failed one, 3 failed.
  function history(workflowId) {
    if (workflowId !== 'image-gen-sdxl') return [];
    const out = [];
    for (let i = 0; i < 12; i++) {
      const failed = i === 2 || i === 7;
      out.push({
        workflow_run_id: 'run_hist' + String(i).padStart(4, '0') + '-0000-4000-8000-000000000000',
        workflow_id: 'image-gen-sdxl', workflow_semantic_version: i < 5 ? '0.9.1' : '0.9.0', status: failed ? 'failed' : 'completed',
        accepted_at_ms: now - (i + 1) * 5 * 3600000, completed_at_ms: now - (i + 1) * 5 * 3600000 + 120000,
        duration_ms: failed ? 21000 : 100000 + i * 3000, selected_runtime_id: 'diffusers', selected_device_id: i % 3 ? 'cuda:1' : 'cuda:0',
        scheduler_policy_id: 'fair-share', retention_policy_id: 'metadata-7d', client_id: 'gui', client_session_id: 's-4f2a', bucket_id: 'default',
        latest_error_code: failed ? 'runtime_model_load_failed' : null,
        last_event_seq: 1000 + i, last_updated_at_ms: now - i * 3600000,
      });
    }
    return out;
  }

  host.invoke = (cmd, args) => {
    const request = (args && args.request) || {};
    switch (cmd) {
      case 'plugin:event|listen':
        return Promise.resolve(1);
      case 'plugin:event|unlisten':
        return Promise.resolve();
      case 'workflow_run_list_query':
        if (!request.workflow_id) return baseInvoke(cmd, args);
        return baseRuns().then((runs) => ({
          runs: runs.filter((r) => r.workflow_id === request.workflow_id).map(detail).concat(history(request.workflow_id)),
          facets: [],
          projection_state: state('run_list', 1842),
        }));
      case 'workflow_run_detail_query':
        return baseRuns().then((runs) => {
          const run = runs.find((r) => r.workflow_run_id === request.workflow_run_id);
          return { run: run ? detail(run) : null, node_statuses: [], projection_state: state('run_detail', 1842), node_projection_state: state('node_status', 1842) };
        });
      case 'workflow_scheduler_timeline_query':
        return baseRuns().then((runs) => {
          const run = runs.find((r) => r.workflow_run_id === request.workflow_run_id);
          return { events: run ? timeline(run) : [], projection_state: state('scheduler_timeline', 1842) };
        });
      case 'workflow_node_status_query':
        return baseRuns().then((runs) => {
          const run = runs.find((r) => r.workflow_run_id === request.workflow_run_id);
          return { nodes: run ? nodes(run) : [], projection_state: state('node_status', 1842) };
        });
      case 'workflow_io_artifact_query':
        return Promise.resolve({
          artifacts: [],
          retention_summary: request.workflow_run_id === FAILED_RUN
            ? [{ retention_state: 'metadata_only', artifact_count: 3 }]
            : [{ retention_state: 'retained', artifact_count: 4 }, { retention_state: 'metadata_only', artifact_count: 1 }],
          projection_state: state('io_artifact', 1842),
        });
      default:
        return baseInvoke(cmd, args);
    }
  };
})();
