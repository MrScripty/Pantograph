<script lang="ts">
  import { onMount } from 'svelte';
  import BaseNode from '../BaseNode.svelte';
  import type {
    NodeDefinition,
    WorkflowMediaFormatOption,
    WorkflowThreeDArtifactFormatSettings,
  } from '../../../services/workflow/types';
  import { workflowService } from '../../../services/workflow/WorkflowService';
  import { nodeExecutionStates, updateNodeData } from '../../../stores/workflowStore';

  interface ThreeDArtifactFormatOverride {
    format_id: string;
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      point_cloud?: {
        positions: number[][];
        colors: number[][];
      };
      artifact_format_override?: ThreeDArtifactFormatOverride | null;
    };
    selected?: boolean;
  }

  interface ThreeDFormatConfig {
    defaults: WorkflowThreeDArtifactFormatSettings;
    formats: WorkflowMediaFormatOption[];
  }

  const DEFAULT_SELECTION_VALUE = '__pantograph_default__';
  let threeDFormatConfigPromise: Promise<ThreeDFormatConfig> | null = null;

  function loadThreeDFormatConfig(): Promise<ThreeDFormatConfig> {
    threeDFormatConfigPromise ??= Promise.all([
      workflowService.artifactFormatSettings(),
      workflowService.artifactFormatCapabilities(),
    ]).then(([settingsResponse, capabilities]) => ({
      defaults: settingsResponse.settings.three_d,
      formats: capabilities.three_d_formats,
    }));
    return threeDFormatConfigPromise;
  }

  let { id, data, selected = false }: Props = $props();

  let defaultFormat = $state<WorkflowThreeDArtifactFormatSettings | null>(null);
  let formatOptions = $state<WorkflowMediaFormatOption[]>([]);
  let formatLoadError = $state<string | null>(null);

  let executionInfo = $derived($nodeExecutionStates.get(id));
  let executionState = $derived(executionInfo?.state || 'idle');
  let hasData = $derived(
    data.point_cloud?.positions && data.point_cloud.positions.length > 0
  );

  const nodeColor = '#14b8a6';

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-teal-500 animate-pulse',
      success: 'bg-teal-500',
      error: 'bg-red-500',
    }[executionState]
  );

  let pointCount = $derived(
    data.point_cloud?.positions?.length ?? 0
  );
  let formatOverride = $derived(normalizeFormatOverride(data.artifact_format_override));
  let selectedFormatId = $derived(formatOverride?.format_id ?? defaultFormat?.format_id ?? '');
  let selectableFormats = $derived(formatOptionItems(formatOptions, selectedFormatId));
  let formatSelectId = $derived(`point-cloud-output-${id}-format`);
  let isUsingDefaultFormat = $derived(!formatOverride);

  let canvasRef: HTMLCanvasElement | undefined = $state();
  let renderError = $state(false);

  function stopControlEvent(event: Event) {
    event.stopPropagation();
  }

  function normalizeFormatOverride(value: unknown): ThreeDArtifactFormatOverride | null {
    if (!value || typeof value !== 'object') return null;
    const record = value as Record<string, unknown>;
    const formatId = typeof record.format_id === 'string' ? record.format_id : '';
    if (!formatId) return null;
    return { format_id: formatId };
  }

  function formatOptionItems(
    options: WorkflowMediaFormatOption[],
    currentValue: string,
  ): WorkflowMediaFormatOption[] {
    if (!currentValue || options.some((option) => option.format_id === currentValue)) {
      return options;
    }
    return [
      {
        format_id: currentValue,
        display_name: `${currentValue} (unsupported)`,
        media_type: 'unknown',
        codec_ids: [],
        quality_min_percent: null,
        quality_max_percent: null,
        bitrate_min_kbps: null,
        bitrate_max_kbps: null,
        crf_min: null,
        crf_max: null,
        bit_depths: [],
        color_profile_ids: [],
        provided_by_dependency_id: 'unknown',
        provided_by_version: null,
      },
      ...options,
    ];
  }

  function updateFormatOverride(override: ThreeDArtifactFormatOverride | null) {
    void updateNodeData(id, { artifact_format_override: override });
  }

  function handleFormatChange(event: Event) {
    const target = event.currentTarget as HTMLSelectElement | null;
    const formatId = target?.value ?? DEFAULT_SELECTION_VALUE;
    if (formatId === DEFAULT_SELECTION_VALUE) {
      updateFormatOverride(null);
      return;
    }
    updateFormatOverride({ format_id: formatId });
  }

  onMount(() => {
    let disposed = false;
    loadThreeDFormatConfig()
      .then((config) => {
        if (disposed) return;
        defaultFormat = config.defaults;
        formatOptions = config.formats;
        formatLoadError = null;
      })
      .catch((error: unknown) => {
        if (disposed) return;
        formatLoadError = error instanceof Error ? error.message : String(error);
      });

    return () => {
      disposed = true;
    };
  });

  $effect(() => {
    if (!canvasRef || !hasData || !data.point_cloud) {
      return;
    }

    let cancelled = false;
    let frameId: number | null = null;
    let removeResizeListener: (() => void) | null = null;
    let cleanupScene: (() => void) | null = null;

    renderError = false;

    void (async () => {
      try {
        const THREE = await import('three');
        const { OrbitControls } = await import('three/addons/controls/OrbitControls.js');
        if (cancelled || !canvasRef || !data.point_cloud) return;

        const height = 160;
        const width = Math.max(canvasRef.clientWidth, 1);

        const scene = new THREE.Scene();
        scene.background = new THREE.Color(0x1a1a1a);

        const camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 1000);
        camera.position.set(0, 0, 5);

        const renderer = new THREE.WebGLRenderer({ canvas: canvasRef, antialias: true });
        renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
        renderer.setSize(width, height, false);

        const controls = new OrbitControls(camera, canvasRef);
        controls.enableDamping = true;

        // Build point cloud geometry
        const positions = data.point_cloud.positions;
        const colors = data.point_cloud.colors;
        const sampleCount = Math.min(positions.length, colors.length);

        const geometry = new THREE.BufferGeometry();
        const posArray = new Float32Array(sampleCount * 3);
        const colArray = new Float32Array(sampleCount * 3);

        for (let i = 0; i < sampleCount; i++) {
          posArray[i * 3] = positions[i][0];
          posArray[i * 3 + 1] = -positions[i][1]; // Flip Y for display
          posArray[i * 3 + 2] = -positions[i][2]; // Flip Z for display
          colArray[i * 3] = colors[i][0];
          colArray[i * 3 + 1] = colors[i][1];
          colArray[i * 3 + 2] = colors[i][2];
        }

        geometry.setAttribute('position', new THREE.BufferAttribute(posArray, 3));
        geometry.setAttribute('color', new THREE.BufferAttribute(colArray, 3));

        const material = new THREE.PointsMaterial({
          size: 0.02,
          vertexColors: true,
        });

        const points = new THREE.Points(geometry, material);
        scene.add(points);

        // Center camera on point cloud
        geometry.computeBoundingSphere();
        if (geometry.boundingSphere) {
          const center = geometry.boundingSphere.center;
          controls.target.copy(center);
          camera.position.set(
            center.x,
            center.y,
            center.z + geometry.boundingSphere.radius * 2
          );
        }

        const handleResize = () => {
          if (!canvasRef) return;
          const nextWidth = Math.max(canvasRef.clientWidth, 1);
          camera.aspect = nextWidth / height;
          camera.updateProjectionMatrix();
          renderer.setSize(nextWidth, height, false);
        };

        window.addEventListener('resize', handleResize);
        removeResizeListener = () => window.removeEventListener('resize', handleResize);

        const animate = () => {
          if (cancelled) return;
          frameId = requestAnimationFrame(animate);
          controls.update();
          renderer.render(scene, camera);
        };
        animate();

        cleanupScene = () => {
          controls.dispose();
          geometry.dispose();
          material.dispose();
          renderer.dispose();
        };
      } catch {
        renderError = true;
      }
    })();

    return () => {
      cancelled = true;
      if (frameId !== null) {
        cancelAnimationFrame(frameId);
      }
      removeResizeListener?.();
      cleanupScene?.();
    };
  });
</script>

<div class="pc-output-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0" style="background-color: {nodeColor}">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 10l-2 1m0 0l-2-1m2 1v2.5M20 7l-2 1m2-1l-2-1m2 1v2.5M14 4l-2-1-2 1M4 7l2-1M4 7l2 1M4 7v2.5M12 21l-2-1m2 1l2-1m-2 1v-2.5M6 18l-2-1v-2.5M18 18l2-1v-2.5" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Point Cloud'}</span>
        <span class="w-2 h-2 rounded-full {statusColor} ml-auto"></span>
      </div>
    {/snippet}

      {#if hasData}
        <div class="space-y-1">
          <canvas
            bind:this={canvasRef}
            class="w-full rounded overflow-hidden nodrag nowheel nopan"
            style="height: 160px; background: #1a1a1a;"
            onpointerdown={(event) => event.stopPropagation()}
            onwheel={(event) => event.stopPropagation()}
          ></canvas>
          <div class="text-[10px] text-neutral-500 text-right">
            {pointCount.toLocaleString()} points
          </div>
          {#if renderError}
            <div class="text-[10px] text-red-400 text-right">
              3D preview unavailable
            </div>
          {/if}
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No point cloud yet
        </div>
      {/if}

      <div class="mt-2 space-y-2 border-t border-neutral-700/70 pt-2">
        <div class="flex items-center justify-between gap-2">
          <label class="text-[10px] text-neutral-400" for={formatSelectId}>Format</label>
          {#if isUsingDefaultFormat && defaultFormat}
            <span class="text-[10px] text-neutral-500">Default {defaultFormat.format_id}</span>
          {:else if formatOverride}
            <span class="text-[10px] text-teal-300">Override</span>
          {/if}
        </div>
        <select
          id={formatSelectId}
          class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200 focus:border-teal-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
          value={formatOverride?.format_id ?? DEFAULT_SELECTION_VALUE}
          disabled={!defaultFormat && !formatOverride}
          onchange={handleFormatChange}
          onmousedown={stopControlEvent}
          onmouseup={stopControlEvent}
          onpointerdown={stopControlEvent}
          onpointerup={stopControlEvent}
          onclickcapture={stopControlEvent}
        >
          <option value={DEFAULT_SELECTION_VALUE}>
            Use default{defaultFormat ? ` (${defaultFormat.format_id})` : ''}
          </option>
          {#each selectableFormats as option}
            <option value={option.format_id}>
              {option.display_name}
            </option>
          {/each}
        </select>

        {#if formatLoadError}
          <div class="text-[10px] text-red-300">{formatLoadError}</div>
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .pc-output-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .pc-output-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }
</style>
