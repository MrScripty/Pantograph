// Settings page (M9) stand-in. Inject after fixture.js: it keeps the base host
// (run list and rejection of everything unmocked) and answers the commands the
// Settings page itself calls: ArtifactStore policy, diagnostics retention
// policy and cleanup, artifact format settings and capabilities, and managed
// media dependencies. It also answers the managed-runtime list and catalog
// refresh commands the embedded runtime panels call with empty lists, so they
// show their empty state; the other embedded app panels (server, model,
// device, RAG, sandbox) fall through to the base host and show their
// unavailable states.
(() => {
  const base = window.__TAURI_INTERNALS__;
  if (!base) throw new Error('fixture-settings.js needs fixture.js injected first');
  const baseInvoke = base.invoke;
  const now = Date.now();
  const GIB = 1073741824;

  let artifactPolicy = {
    policy_id: 'artifact-local-v1',
    policy_version: 4,
    ttl_seconds: 604800,
    max_disk_bytes: 50 * GIB,
    max_memory_bytes: 2 * GIB,
    max_single_artifact_bytes: null,
    spill_threshold_bytes: 64 * 1048576,
    delete_on_consume: false,
  };

  const scope = (days, mode) => ({ retention_days: days, payload_mode: mode });
  let retentionPolicy = {
    policy_id: 'standard-local-v1',
    policy_version: 3,
    retention_class: 'standard',
    retention_days: 30,
    settings: {
      final_outputs: scope(30, 'retain_payload_reference'),
      workflow_inputs: scope(30, 'retain_payload_reference'),
      intermediate_node_io: scope(14, 'metadata_only'),
      failed_run_data: scope(30, 'retain_payload_reference'),
      max_artifact_bytes: null,
      max_total_storage_bytes: null,
      media_behavior: 'metadata_and_reference_only',
      compression_behavior: 'not_configured',
      cleanup_trigger: 'manual_or_maintenance',
    },
    applied_at_ms: now - 3 * 86400000,
    explanation: 'Keep run outputs and inputs for a month; node I/O metadata for two weeks.',
  };

  let formatSettings = {
    image: { format_id: 'jpg', quality_percent: 85, color_profile_id: 'srgb' },
    audio: { container_id: 'ogg', codec_id: 'opus', bitrate_kbps: 96 },
    video: { container_id: 'ivf', codec_id: 'svt_av1', crf: 32, bit_depth: '8bit' },
    three_d: { format_id: 'glb' },
  };

  const opt = (id, name, type, extra) => Object.assign({
    format_id: id, display_name: name, media_type: type, codec_ids: [],
    quality_min_percent: null, quality_max_percent: null, bitrate_min_kbps: null, bitrate_max_kbps: null,
    crf_min: null, crf_max: null, bit_depths: [], color_profile_ids: [],
    provided_by_dependency_id: 'ffmpeg', provided_by_version: null,
  }, extra || {});
  const capabilities = {
    image_formats: [
      opt('jpg', 'JPEG', 'image/jpeg', { quality_min_percent: 1, quality_max_percent: 100, color_profile_ids: ['srgb', 'display_p3'], provided_by_dependency_id: 'oiiotool' }),
      opt('png', 'PNG', 'image/png', { color_profile_ids: ['srgb'], provided_by_dependency_id: 'oiiotool' }),
      opt('exr', 'OpenEXR', 'image/x-exr', { color_profile_ids: ['acescg', 'linear_srgb'], provided_by_dependency_id: 'oiiotool' }),
    ],
    audio_formats: [
      opt('ogg', 'Ogg', 'audio/ogg', { codec_ids: ['opus', 'vorbis'], bitrate_min_kbps: 32, bitrate_max_kbps: 512 }),
      opt('wav', 'WAV', 'audio/wav', { codec_ids: ['pcm'] }),
    ],
    video_formats: [
      opt('ivf', 'AV1 IVF', 'video/av1', { codec_ids: ['svt_av1'], crf_min: 0, crf_max: 63, bit_depths: ['8bit', '10bit'] }),
    ],
    three_d_formats: [
      opt('glb', 'GLB', 'model/gltf-binary', { provided_by_dependency_id: 'pantograph-3d' }),
      opt('gltf', 'glTF', 'model/gltf+json', { provided_by_dependency_id: 'pantograph-3d' }),
    ],
  };

  const dep = (id, name, owner, project, catalogVersion, files, installed) => {
    const category = id === 'open_color_io' ? 'native_library_artifact' : 'tool_binary';
    const versions = installed.map((v, i) => ({
      version: v, platform_key: 'linux-x86_64', install_root: '/home/user/.local/share/pantograph/managed-media/' + id + '/' + v,
      expected_files: files, missing_files: [], install_state: 'installed', readiness: 'ready', selected: i === 0, active: i === 0,
    }));
    const ready = installed.length > 0;
    return {
      id, display_name: name, category,
      install_state: ready ? 'installed' : 'missing', readiness: ready ? 'ready' : 'missing', available: ready,
      missing_files: ready ? [] : files,
      catalog: {
        id, display_name: name, category, source: { owner, project },
        license_redistribution: 'BSD-3-Clause', platform_key: 'linux-x86_64', version: catalogVersion,
        package_kind: id === 'open_color_io' ? 'native_package' : 'archive', archive_kind: id === 'open_color_io' ? null : 'tar_gz',
        archive_name: null, download_url: null, expected_files: files, checksum_sha256: null, signature: null,
      },
      selection: {
        selected_version: installed[0] || null, active_version: installed[0] || null, default_version: installed[0] || null,
      },
      versions,
    };
  };
  let dependencies = [
    dep('ffmpeg', 'FFmpeg', 'FFmpeg', 'FFmpeg', '7.1', ['bin/ffmpeg'], ['7.1', '7.0.2']),
    dep('ocioconvert', 'ocioconvert', 'AcademySoftwareFoundation', 'OpenColorIO', '2.4.2', ['bin/ocioconvert'], []),
    dep('oiiotool', 'oiiotool', 'AcademySoftwareFoundation', 'OpenImageIO', '3.0.0', ['bin/oiiotool'], ['3.0.0']),
    dep('open_color_io', 'OpenColorIO', 'AcademySoftwareFoundation', 'OpenColorIO', '2.4.2', ['lib/OpenColorIO'], []),
  ];

  window.__TAURI_INTERNALS__ = Object.assign({}, base, {
    invoke: (cmd, args) => {
      const request = (args && args.request) || {};
      switch (cmd) {
        case 'workflow_artifact_policy':
          return Promise.resolve(artifactPolicy);
        case 'workflow_update_artifact_policy':
          artifactPolicy = Object.assign({}, args.policy, { policy_version: artifactPolicy.policy_version + 1 });
          return Promise.resolve(artifactPolicy);
        case 'workflow_retention_policy_query':
          return Promise.resolve({ retention_policy: retentionPolicy });
        case 'workflow_retention_policy_update':
          retentionPolicy = Object.assign({}, retentionPolicy, {
            policy_version: retentionPolicy.policy_version + 1,
            retention_days: request.retention_days,
            explanation: request.explanation,
            applied_at_ms: Date.now(),
          });
          return Promise.resolve({ retention_policy: retentionPolicy });
        case 'workflow_retention_cleanup_apply':
          return Promise.resolve({
            cleanup: {
              policy_id: retentionPolicy.policy_id,
              policy_version: retentionPolicy.policy_version,
              retention_class: retentionPolicy.retention_class,
              cutoff_occurred_before_ms: Date.now() - retentionPolicy.retention_days * 86400000,
              expired_artifact_count: 12,
              last_event_seq: 4417,
            },
          });
        case 'workflow_artifact_format_settings':
          return Promise.resolve({ settings: formatSettings });
        case 'workflow_update_artifact_format_settings':
          formatSettings = request.settings;
          return Promise.resolve({ settings: formatSettings });
        case 'workflow_artifact_format_capabilities':
          return Promise.resolve(capabilities);
        case 'workflow_list_managed_media_dependencies':
          return Promise.resolve(dependencies);
        case 'workflow_managed_media_dependency_status':
          return Promise.resolve(dependencies.find((d) => d.id === args.id));
        case 'list_managed_runtimes':
        case 'refresh_managed_runtime_catalogs':
        case 'list_managed_dependencies':
          return Promise.resolve([]);
        default:
          return baseInvoke(cmd, args);
      }
    },
  });
})();
