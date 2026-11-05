// Scheduler (M1) stand-in. Inject after fixture.js: it reuses that file's runs
// (through its workflow_run_list_query answer), adds the scheduler facts the
// page shows (session, policy, estimates, placement), and answers the
// selected-run queries the Scheduler makes: scheduler timeline, scheduler
// estimate and I/O artifacts (for the retention summary). Every other command
// falls through to fixture.js, which rejects it like an unreachable host.
// `window.__fixtureCalls` counts invocations per command for walkthrough checks.
(() => {
  const host = window.__TAURI_INTERNALS__;
  const baseInvoke = host.invoke;
  const now = Date.now();
  const min = 60000;
  const state = (name) => ({ projection_name: name, projection_version: 1, last_applied_event_seq: 41, status: 'current', rebuilt_at_ms: null, updated_at_ms: now });
  const defaults = { scheduler_policy_id: 'fair-share', retention_policy_id: 'keep-7d', bucket_id: 'default', client_id: 'gui', client_session_id: 's-4f2a', workflow_execution_session_id: 'wes-gui-1', selected_backend_key: null, accepted_at_ms: now - 20 * min, scheduler_priority: 0 };
  // Extra facts per run id prefix (the base fixture's runs, by position).
  const extras = {
    run_7c9e6679: { selected_backend_key: 'llama_cpp', selected_device_class: 'cuda', started_at_ms: now - 4 * min, estimated_duration_ms: 6 * min, estimate_confidence: 'high', model_cache_state: 'loaded' },
    run_1b4e28ba: { selected_backend_key: 'diffusers', selected_device_class: 'cuda', started_at_ms: now - 2 * min, estimated_duration_ms: 2 * min },
    run_6fa459ea: { client_id: 'cli', client_session_id: 's-9c01', scheduler_priority: 5, selected_runtime_id: 'whisper', estimated_queue_wait_ms: 3 * min, estimated_duration_ms: 4 * min, estimate_confidence: 'medium' },
    run_886313e1: { selected_runtime_id: 'llama.cpp', selected_device_id: 'cuda:1', scheduler_reason: 'next on cuda:1, model cached', estimated_queue_wait_ms: 2 * min, estimated_duration_ms: 6 * min, estimate_confidence: 'high', model_cache_state: 'cache_hit' },
    run_a8098c1a: { selected_runtime_id: 'llama.cpp', workflow_execution_session_id: null, scheduler_reason: 'cold load, 12 s', estimated_duration_ms: 9 * min, model_cache_state: 'cache_miss' },
    run_16fd2706: { selected_runtime_id: 'llama.cpp', selected_device_id: 'cuda:0', accepted_at_ms: now - 12 * min, started_at_ms: now - 11 * min },
    run_3f2504e0: { selected_runtime_id: 'diffusers', selected_device_id: 'cuda:1', accepted_at_ms: now - 32 * min, started_at_ms: now - 31 * min, duration_ms: 22000, latest_error_code: 'runtime_oom' },
  };
  const extraFor = (id) => extras[id.slice(0, 12)] || {};
  const enrich = (response) => ({ ...response, runs: response.runs.map((run) => ({ ...defaults, ...run, ...extraFor(run.workflow_run_id) })) });
  let runsCache = null;
  const loadRuns = () => (runsCache ??= baseInvoke('workflow_run_list_query', { request: { limit: 250 } }).then(enrich));
  const findRun = (id) => loadRuns().then((response) => response.runs.find((run) => run.workflow_run_id === id) || null);

  let seq = 0;
  const event = (run, at, kind, source, summary, extra) => ({
    event_seq: ++seq, event_id: 'evt-' + seq, event_kind: kind, source_component: source, occurred_at_ms: at, recorded_at_ms: at,
    workflow_run_id: run.workflow_run_id, workflow_id: run.workflow_id, workflow_semantic_version: run.workflow_semantic_version,
    scheduler_policy_id: run.scheduler_policy_id, retention_policy_id: run.retention_policy_id, summary, detail: null, payload_json: '{}', ...(extra || {}),
  });
  const timelineFor = (run) => {
    const accepted = run.accepted_at_ms || now - 20 * min;
    const events = [
      event(run, accepted, 'run_accepted', 'workflow_service', `Accepted from ${run.client_id}, session ${run.client_session_id}`),
      event(run, accepted + 2000, 'scheduler_queue_placement', 'scheduler', `Enqueued under ${run.scheduler_policy_id}, priority ${run.scheduler_priority}`),
    ];
    if (run.selected_device_id) {
      events.push(event(run, accepted + 26000, 'scheduler_queue_placement', 'scheduler', `Placed on ${run.selected_device_id} with ${run.selected_runtime_id}`, { scheduler_attempt_id: 'attempt-1', scheduler_attempt_runtime_id: run.selected_runtime_id, scheduler_attempt_device_id: run.selected_device_id }));
    }
    if (run.status === 'delayed') events.push(event(run, now - 5 * min, 'scheduler_queue_placement', 'scheduler', 'Delayed: ' + run.scheduler_reason));
    if (run.started_at_ms) events.push(event(run, run.started_at_ms, 'run_started', 'runtime', 'Started'));
    if (run.status === 'completed') events.push(event(run, run.completed_at_ms, 'run_completed', 'runtime', 'Finished, artifacts kept 7 days'));
    if (run.status === 'failed') {
      const failed = event(run, run.completed_at_ms, 'run_failed', 'runtime', 'Failed in load_model');
      failed.detail = run.latest_error_message;
      events.push(failed);
    }
    return events;
  };

  const calls = (window.__fixtureCalls = {});
  host.invoke = (cmd, args, options) => {
    calls[cmd] = (calls[cmd] || 0) + 1;
    const request = (args && args.request) || {};
    switch (cmd) {
      case 'workflow_run_list_query':
        return loadRuns();
      case 'workflow_scheduler_timeline_query':
        return findRun(request.workflow_run_id).then((run) => ({ events: run ? timelineFor(run) : [], projection_state: state('scheduler_timeline') }));
      case 'workflow_scheduler_estimate_query':
        return findRun(request.workflow_run_id).then((run) => ({
          estimate: run && (run.estimated_duration_ms || run.estimated_queue_wait_ms) ? {
            workflow_run_id: run.workflow_run_id, workflow_id: run.workflow_id, scheduler_policy_id: run.scheduler_policy_id,
            estimate_confidence: run.estimate_confidence || null, estimated_queue_wait_ms: run.estimated_queue_wait_ms || null,
            estimated_duration_ms: run.estimated_duration_ms || null, model_cache_state: run.model_cache_state || null,
            last_event_seq: 40, last_updated_at_ms: now,
          } : null,
          projection_state: state('run_detail'),
        }));
      case 'workflow_io_artifact_query':
        return findRun(request.workflow_run_id).then((run) => ({
          artifacts: [],
          retention_summary: run && run.started_at_ms ? [{ retention_state: 'retained', artifact_count: 3 }, { retention_state: 'metadata_only', artifact_count: 1 }] : [],
          projection_state: state('io_artifact'),
        }));
      default:
        return baseInvoke(cmd, args, options);
    }
  };
})();
