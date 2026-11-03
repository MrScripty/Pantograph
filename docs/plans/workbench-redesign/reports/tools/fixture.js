// Stand-in Tauri host: answers workflow_run_list_query with fixture rows and
// rejects every other command like an unreachable host would.
(() => {
  const now = Date.now();
  const mk = (id, wf, ver, status, extra) => Object.assign({ workflow_run_id: id, workflow_id: wf, workflow_semantic_version: ver, status, last_event_seq: 1, last_updated_at_ms: now - 60000 }, extra || {});
  const runs = [
    mk('run_7c9e6679-7425-40de-944b-e07fc1f90ae7', 'ingest-docs-rag', '1.4.0', 'running', { selected_runtime_id: 'llama.cpp', selected_device_id: 'cuda:0' }),
    mk('run_1b4e28ba-2fa1-11d2-883f-0016d3cca427', 'image-gen-sdxl', '0.9.2', 'running', { selected_runtime_id: 'diffusers', selected_device_id: 'cuda:1' }),
    mk('run_6fa459ea-ee8a-3ca4-894e-db77e160355e', 'transcribe-meeting', '2.1.0', 'delayed', { scheduler_reason: 'needs 8 GB on cuda:0, 6.8 GB free' }),
    mk('run_886313e1-3b8a-5372-9b90-0c9aee199e5d', 'summarize-daily', '1.0.3', 'queued', { scheduler_queue_position: 2 }),
    mk('run_a8098c1a-f86e-11da-bd1a-00112444be1e', 'embed-corpus', '1.4.0', 'queued', { scheduler_queue_position: 3 }),
    mk('run_16fd2706-8baf-433b-82eb-8c7fada847da', 'summarize-daily', '1.0.3', 'completed', { completed_at_ms: now - 600000, duration_ms: 38000 }),
    mk('run_3f2504e0-4f89-11d3-9a0c-0305e82c3301', 'image-gen-sdxl', '0.9.1', 'failed', { completed_at_ms: now - 1800000, latest_error_message: 'out of memory, needed 14.2 GB' }),
  ];
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd) => cmd === 'workflow_run_list_query'
      ? Promise.resolve({ runs, facets: [], projection_state: { projection_name: 'run_list', projection_version: 1, last_applied_event_seq: 1, status: 'current', rebuilt_at_ms: null, updated_at_ms: now } })
      : Promise.reject(new Error('fixture host: ' + cmd + ' is not mocked')),
    transformCallback: () => 0,
    unregisterCallback: () => {},
    convertFileSrc: (p) => p,
  };
})();
