// M7 Library stand-in. Inject after fixture.js (which installs the run list):
// wraps its invoke and answers the commands LibraryPage calls. Shapes follow
// the host: Pumas selector rows as PortOption (crates/workflow-nodes/src/input/
// puma_lib.rs), usage rows as LibraryUsageProjectionRecord, HF results as the
// camelCase pumas HuggingFaceModel. No sizes or device residency: the
// projections do not carry them.
(() => {
  const base = window.__TAURI_INTERNALS__;
  const now = Date.now();
  const day = 24 * 60 * 60 * 1000;
  const runIngest = 'run_7c9e6679-7425-40de-944b-e07fc1f90ae7';
  const runSdxl = 'run_3f2504e0-4f89-11d3-9a0c-0305e82c3301';
  const runSummary = 'run_16fd2706-8baf-433b-82eb-8c7fada847da';
  const opt = (id, label, meta, extra) => Object.assign({
    value: { model_id: id },
    label,
    description: (meta.model_type || 'unknown'),
    metadata: Object.assign({ id, selector_snapshot_contract_version: 1, selector_row_executable: true }, meta),
  }, extra || {});
  const options = [
    opt('embedding/baai/bge-m3', 'bge-m3', { model_type: 'embedding', repo_id: 'BAAI/bge-m3', pipeline_tag: 'feature-extraction', recommended_backend: 'llama.cpp', display_entry_path: '/models/embedding/baai/bge-m3/bge-m3-q8_0.gguf', storage_kind: 'library_owned', validation_state: 'valid' }),
    opt('diffusion/stabilityai/sdxl-base-1.0', 'sdxl-base-1.0', { model_type: 'diffusion', repo_id: 'stabilityai/stable-diffusion-xl-base-1.0', task_type_primary: 'image_generation', pipeline_tag: 'text-to-image', recommended_backend: 'diffusers', display_entry_path: '/models/diffusion/stabilityai/sdxl-base-1.0', storage_kind: 'library_owned' }),
    opt('llm/qwen/qwen3-8b-instruct', 'qwen3-8b-instruct', { model_type: 'llm', repo_id: 'Qwen/Qwen3-8B-Instruct-GGUF', task_type_primary: 'text-generation', recommended_backend: 'llama.cpp', display_entry_path: '/models/llm/qwen/qwen3-8b-instruct/qwen3-8b-q5_k_m.gguf', storage_kind: 'library_owned' }),
    opt('audio/systran/faster-whisper-large-v3', 'whisper-large-v3', { model_type: 'audio', repo_id: 'Systran/faster-whisper-large-v3', pipeline_tag: 'automatic-speech-recognition', runtime_engine_hints: ['whisper'], display_entry_path: '/models/audio/systran/faster-whisper-large-v3' }),
    opt('audio/kittenml/kitten-tts-nano', 'kitten-tts', { model_type: 'audio', repo_id: 'KittenML/kitten-tts-nano', pipeline_tag: 'text-to-speech', runtime_engine_hints: ['onnx-runtime'], display_entry_path: '/models/audio/kittenml/kitten-tts-nano' }),
    opt('diffusion/rundiffusion/juggernaut-x-v10', 'juggernaut-x-v10', { model_type: 'diffusion', repo_id: 'RunDiffusion/Juggernaut-X-v10', task_type_primary: 'image_generation', recommended_backend: 'diffusers', display_indexed_path: '/models/diffusion/rundiffusion/juggernaut-x-v10' }),
    opt('vlm/qwen/qwen2.5-vl-7b', 'qwen2.5-vl-7b', { model_type: 'vlm', repo_id: 'Qwen/Qwen2.5-VL-7B-Instruct', entry_path_state: 'missing' },
      { disabled: true, unavailableState: 'missing_model_facts', unavailableReason: 'Pumas selector row is not ready: entry path is Missing, artifact is Missing' }),
  ];
  const use = (asset, at, extra) => Object.assign({
    asset_id: asset, total_access_count: 1, run_access_count: 0, total_network_bytes: 0,
    last_accessed_at_ms: at, last_operation: 'run_usage', last_cache_status: null,
    last_workflow_run_id: null, last_workflow_id: null, last_workflow_version_id: null, last_workflow_semantic_version: null,
    last_client_id: null, last_client_session_id: null, last_bucket_id: null, last_event_seq: 40, last_updated_at_ms: at,
  }, extra || {});
  let seq = 48;
  const assets = [
    use('pumas://models/embedding/baai/bge-m3', now - 4 * 60000, { total_access_count: 41, run_access_count: 41, last_workflow_run_id: runIngest, last_workflow_id: 'ingest-docs-rag', last_workflow_semantic_version: '1.4.0', last_event_seq: 47 }),
    use('pumas://models/diffusion/stabilityai/sdxl-base-1.0', now - 30 * 60000, { total_access_count: 18, run_access_count: 18, last_workflow_run_id: runSdxl, last_workflow_id: 'image-gen-sdxl', last_workflow_semantic_version: '0.9.1', last_event_seq: 45 }),
    use('pumas://models/llm/qwen/qwen3-8b-instruct', now - 10 * 60000, { total_access_count: 26, run_access_count: 26, last_workflow_run_id: runSummary, last_workflow_id: 'summarize-daily', last_workflow_semantic_version: '1.0.3', last_event_seq: 46 }),
    use('pumas://models/diffusion/rundiffusion/juggernaut-x-v10', now - 34 * day, { total_access_count: 4, run_access_count: 4, last_workflow_run_id: 'run_0d3c1e2a-0000-4000-8000-00000000a001', last_workflow_id: 'juggernaut-x-v10-sdxl', last_workflow_semantic_version: '0.2.0', last_event_seq: 12 }),
    use('hf://models/BAAI/bge-m3', now - 50 * day, { last_operation: 'download', last_event_seq: 3 }),
    use('hf://models', now - 2 * day, { last_operation: 'search', total_access_count: 6, last_event_seq: 30 }),
  ];
  const hf = [
    { repoId: 'Qwen/Qwen3-14B-Instruct-GGUF', name: 'Qwen3-14B-Instruct-GGUF', developer: 'Qwen', kind: 'text-generation', formats: ['gguf'], quants: ['Q4_K_M'], downloadOptions: [], url: '', downloads: 120400, totalSizeBytes: 9663676416 },
    { repoId: 'BAAI/bge-reranker-v2-m3', name: 'bge-reranker-v2-m3', developer: 'BAAI', kind: 'text-ranking', formats: ['safetensors'], quants: [], downloadOptions: [], url: '', downloads: 840000, totalSizeBytes: 1181116006 },
    { repoId: 'nomic-ai/nomic-embed-text-v2-moe', name: 'nomic-embed-text-v2-moe', developer: 'nomic-ai', kind: 'feature-extraction', formats: ['safetensors'], quants: [], downloadOptions: [], url: '', downloads: 31000 },
    { repoId: 'stabilityai/sd-turbo', name: 'sd-turbo', developer: 'stabilityai', kind: 'text-to-image', formats: ['safetensors'], quants: [], downloadOptions: [], url: '', downloads: 204000, totalSizeBytes: 2791728742 },
    { repoId: 'openai/whisper-large-v3-turbo', name: 'whisper-large-v3-turbo', developer: 'openai', kind: 'automatic-speech-recognition', formats: ['safetensors'], quants: [], downloadOptions: [], url: '', downloads: 1100000, totalSizeBytes: 1717986918 },
  ];
  const state = () => ({ projection_name: 'library_usage', projection_version: 1, last_applied_event_seq: seq, status: 'current', rebuilt_at_ms: null, updated_at_ms: now });
  window.__libraryFixtureCalls = [];
  window.__TAURI_INTERNALS__ = Object.assign({}, base, {
    invoke: (cmd, args) => {
      window.__libraryFixtureCalls.push([cmd, args]);
      switch (cmd) {
        case 'query_port_options':
          return Promise.resolve({ options, totalCount: options.length, searchable: true, metadata: { selector_snapshot_contract_version: 1 } });
        case 'workflow_library_usage_query':
          return Promise.resolve({ assets, projection_state: state() });
        case 'search_hf_models_with_audit':
          seq += 1;
          return Promise.resolve({ models: hf.slice(0, (args && args.limit) || 25), auditEventSeq: seq });
        case 'start_hf_download_with_audit':
          seq += 1;
          return Promise.resolve({ downloadId: 'dl-' + seq, auditEventSeq: seq });
        case 'delete_pumas_model_with_audit':
          seq += 1;
          return Promise.resolve({ success: true, error: null, auditEventSeq: seq });
        case 'plugin:event|listen':
          return Promise.resolve(1);
        case 'plugin:event|unlisten':
          return Promise.resolve();
        default:
          return base.invoke(cmd, args);
      }
    },
  });
})();
