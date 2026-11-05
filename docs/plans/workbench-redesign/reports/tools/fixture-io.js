// I/O Inspector fixture (M3). Load after fixture.js: it keeps that file's run
// list and answers the commands the I/O Inspector calls, so the walkthrough can
// screenshot the populated layout with an image, a text and a metadata-only
// artifact, plus the saved-workflow disclosure. Evidence tooling only.
(() => {
  const host = window.__TAURI_INTERNALS__;
  const baseInvoke = host.invoke;
  const now = Date.now();
  const state = (name) => ({ projection_name: name, projection_version: 1, last_applied_event_seq: 412, status: 'current', rebuilt_at_ms: null, updated_at_ms: now });
  const node = (id, type, x, y) => ({ id, node_type: type, position: { x, y }, data: {} });
  const edge = (id, source, sourceHandle, target, targetHandle) => ({ id, source, source_handle: sourceHandle, target, target_handle: targetHandle });
  const nodes = [node('prompt', 'text-input', 0, 40), node('load_model', 'puma-lib', 0, 200), node('sample', 'pytorch-inference', 280, 120), node('decode', 'image-output', 560, 120), node('unload', 'unload-model', 840, 120)];
  const edges = [edge('e1', 'prompt', 'text', 'sample', 'prompt'), edge('e2', 'load_model', 'model_path', 'sample', 'model_ref'), edge('e3', 'sample', 'latents', 'decode', 'latents'), edge('e4', 'decode', 'image', 'unload', 'trigger')];
  const diagnostics = [{ code: 'retired_node_type', severity: 'warning', scope: 'node', node_id: 'unload', node_type: 'unload-model', message: 'unload-model was retired after this run; the captured graph still lists it.', blocking_submission: false }];
  const prompt = 'A pantograph on a drafting table, brass fittings, soft north light,\nwide shot, 35 mm film grain, muted palette';
  const modelPath = '{\n  "path": "/models/sdxl-base-1.0",\n  "variant": "fp16",\n  "cache": "hit"\n}';
  const enc = (text) => Array.from(new TextEncoder().encode(text));
  let png = null;
  const pngBytes = async () => {
    if (png) return png;
    const canvas = document.createElement('canvas');
    canvas.width = 1024; canvas.height = 1024;
    const g = canvas.getContext('2d');
    const bg = g.createLinearGradient(0, 0, 0, 1024); bg.addColorStop(0, '#3A3F47'); bg.addColorStop(1, '#2A2E35');
    g.fillStyle = bg; g.fillRect(0, 0, 1024, 1024);
    const glow = (x, y, r, c) => { const rg = g.createRadialGradient(x, y, 0, x, y, r); rg.addColorStop(0, c); rg.addColorStop(1, 'rgba(0,0,0,0)'); g.fillStyle = rg; g.fillRect(0, 0, 1024, 1024); };
    glow(320, 400, 520, 'rgba(217,176,106,0.45)'); glow(720, 660, 460, 'rgba(159,183,217,0.38)');
    g.strokeStyle = 'rgba(228,230,234,0.55)'; g.lineWidth = 6;
    g.beginPath(); g.moveTo(220, 760); g.lineTo(460, 420); g.lineTo(700, 640); g.lineTo(820, 300); g.stroke();
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
    png = Array.from(new Uint8Array(await blob.arrayBuffer()));
    return png;
  };
  const artifact = (id, extra) => Object.assign({ event_seq: 400, event_id: 'ev-' + id, occurred_at_ms: now - 90000, recorded_at_ms: now - 90000, workflow_run_id: '', workflow_id: 'image-gen-sdxl', workflow_semantic_version: '0.9.2', artifact_id: id, payload_artifact_id: id, artifact_fact_id: 'fact-' + id, retention_state: 'retained', lifecycle_state: 'retained', access_modes: ['read', 'download'], read_handle: 'artifact-read://' + id, runtime_id: 'diffusers', runtime_version: '0.31', selected_backend_key: 'cuda', model_id: 'pumas://models/sdxl-base-1.0' }, extra);
  const artifacts = (runId) => [
    artifact('art_7f21a0', { workflow_run_id: runId, artifact_role: 'workflow_input', node_id: 'prompt', node_type: 'text-input', producer_node_id: 'prompt', producer_port_id: 'text', consumer_node_id: 'sample', consumer_port_id: 'prompt', media_type: 'text/plain', payload_kind: 'text', size_bytes: enc(prompt).length, content_hash: 'sha256:9b1c4e…e4a0', runtime_id: null, runtime_version: null, selected_backend_key: null, model_id: null, format: { format_id: 'utf-8', media_type: 'text/plain' } }),
    artifact('art_7f21a1', { workflow_run_id: runId, artifact_role: 'node_output', node_id: 'load_model', node_type: 'puma-lib', producer_node_id: 'load_model', producer_port_id: 'model_path', consumer_node_id: 'sample', consumer_port_id: 'model_ref', media_type: 'application/json', payload_kind: 'structured', size_bytes: enc(modelPath).length, content_hash: 'sha256:51d0a2…77c2', format: { format_id: 'json', media_type: 'application/json' } }),
    artifact('art_7f21a4', { workflow_run_id: runId, artifact_role: 'node_output', node_id: 'sample', node_type: 'pytorch-inference', producer_node_id: 'sample', producer_port_id: 'latents', consumer_node_id: 'decode', consumer_port_id: 'latents', media_type: 'application/octet-stream', payload_kind: 'generic_binary', size_bytes: 35232153, retention_state: 'metadata_only', retention_reason: 'Intermediate node I/O keeps metadata only under the standard policy.', access_modes: [], read_handle: null, content_hash: null, format: { format_id: 'safetensors', media_type: 'application/octet-stream' } }),
    artifact('art_7f21a5', { workflow_run_id: runId, artifact_role: 'workflow_output', node_id: 'decode', node_type: 'image-output', producer_node_id: 'decode', producer_port_id: 'image', media_type: 'image/png', payload_kind: 'image', size_bytes: 4299161, content_hash: 'sha256:c7e2f0…b8d1', format: { format_id: 'png', media_type: 'image/png', bit_depth: '8-bit', color_profile_id: 'sRGB' } }),
  ];
  const resolved = (nodeId, portId, direction, artifactId, up) => Object.assign({ node_id: nodeId, port_id: portId, direction, resolution: direction === 'output' ? 'produced_output' : 'derived_from_edge', artifact_id: artifactId, payload_artifact_id: artifactId }, up ? { provenance_kind: 'graph_edge', upstream_node_id: up[0], upstream_port_id: up[1] } : {});
  const inspection = (runId) => {
    const ios = artifacts(runId);
    return {
      run_graph: { workflow_run_id: runId, workflow_id: 'image-gen-sdxl', workflow_version_id: 'wv-1', workflow_presentation_revision_id: 'pr-1', workflow_semantic_version: '0.9.2', workflow_execution_fingerprint: 'fp', snapshot_created_at_ms: now - 120000, workflow_version_created_at_ms: now - 864e5, presentation_revision_created_at_ms: now - 864e5, graph: { nodes, edges }, graph_diagnostics: diagnostics, executable_topology: { schema_version: 1, nodes: [], edges: [] }, presentation_metadata: { schema_version: 1, nodes: [], edges: [] }, graph_settings: { schema_version: 1, nodes: [] } },
      run: null,
      node_statuses: [['prompt', 'completed'], ['load_model', 'completed'], ['sample', 'completed'], ['decode', 'completed'], ['unload', 'running']].map(([id, status]) => ({ workflow_run_id: runId, workflow_id: 'image-gen-sdxl', node_id: id, status })),
      io_artifacts: ios,
      resolved_node_io: [
        resolved('prompt', 'text', 'output', 'art_7f21a0'),
        resolved('load_model', 'model_path', 'output', 'art_7f21a1'),
        resolved('sample', 'prompt', 'input', 'art_7f21a0', ['prompt', 'text']),
        resolved('sample', 'model_ref', 'input', 'art_7f21a1', ['load_model', 'model_path']),
        resolved('sample', 'latents', 'output', 'art_7f21a4'),
        resolved('decode', 'latents', 'input', 'art_7f21a4', ['sample', 'latents']),
        resolved('decode', 'image', 'output', 'art_7f21a5'),
      ],
      retention_summary: [{ retention_state: 'retained', artifact_count: 3 }, { retention_state: 'metadata_only', artifact_count: 1 }],
      run_projection_state: state('run_detail'), node_projection_state: state('node_status'), io_projection_state: state('io_artifact'),
    };
  };
  const descriptor = (id) => {
    const a = artifacts('').find((x) => x.artifact_id === id);
    return { artifact: a && { artifact_id: id, payload_kind: a.payload_kind, lifecycle_state: a.lifecycle_state, retention_state: a.retention_state, byte_length: a.size_bytes, content_hash: a.content_hash, format: a.format, attribution: {}, access_modes: a.access_modes, read_handle: a.read_handle, stream_handle: null } };
  };
  const body = async (id) => {
    const bytes = id === 'art_7f21a5' ? await pngBytes() : id === 'art_7f21a0' ? enc(prompt) : enc(modelPath);
    const media = id === 'art_7f21a5' ? 'image/png' : id === 'art_7f21a0' ? 'text/plain' : 'application/json';
    return { response: { artifact_id: id, media_type: media, body_transport: 'binary_body', read_handle: 'artifact-read://' + id, byte_length: bytes.length, content_hash: 'sha256:fixture', complete: true }, body: bytes };
  };
  host.invoke = async (cmd, args) => {
    const request = (args && args.request) || {};
    switch (cmd) {
      case 'plugin:event|listen': return 1;
      case 'plugin:event|unlisten': return null;
      case 'workflow_run_inspection_query': return inspection(request.workflow_run_id);
      case 'workflow_artifact_descriptor': return descriptor(request.artifact_id);
      case 'workflow_read_artifact_body': return body(request.artifact_id);
      case 'workflow_acknowledge_artifact_consumed': return { artifact_id: request.artifact_id, retained_after_consume: true };
      case 'list_workflows': return [{ id: 'image-gen-sdxl', name: 'image-gen-sdxl', description: '', created: new Date(now - 864e6).toISOString(), modified: new Date(now - 864e5).toISOString() }];
      case 'workflow_graph_inspect': return { graph: { nodes, edges }, selected_node: null, diagnostics, run_context: null };
      default: return baseInvoke(cmd, args);
    }
  };
})();
