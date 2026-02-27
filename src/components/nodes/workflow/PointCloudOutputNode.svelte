<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      point_cloud?: {
        positions: number[][];
        colors: number[][];
      };
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

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

  // Three.js canvas reference — lazily initialized
  let canvasContainer: HTMLDivElement | undefined = $state();
  let threeInitialized = $state(false);

  $effect(() => {
    if (canvasContainer && hasData && !threeInitialized) {
      initThreeScene();
    }
  });

  async function initThreeScene() {
    if (!canvasContainer || !data.point_cloud) return;
    try {
      const THREE = await import('three');
      const { OrbitControls } = await import('three/addons/controls/OrbitControls.js');

      const width = canvasContainer.clientWidth;
      const height = 160;

      const scene = new THREE.Scene();
      scene.background = new THREE.Color(0x1a1a1a);

      const camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 1000);
      camera.position.set(0, 0, 5);

      const renderer = new THREE.WebGLRenderer({ antialias: true });
      renderer.setSize(width, height);
      canvasContainer.innerHTML = '';
      canvasContainer.appendChild(renderer.domElement);

      const controls = new OrbitControls(camera, renderer.domElement);
      controls.enableDamping = true;

      // Build point cloud geometry
      const positions = data.point_cloud.positions;
      const colors = data.point_cloud.colors;

      const geometry = new THREE.BufferGeometry();
      const posArray = new Float32Array(positions.length * 3);
      const colArray = new Float32Array(colors.length * 3);

      for (let i = 0; i < positions.length; i++) {
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

      function animate() {
        requestAnimationFrame(animate);
        controls.update();
        renderer.render(scene, camera);
      }
      animate();

      threeInitialized = true;
    } catch {
      // Three.js not available — show fallback
    }
  }
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

    {#snippet children()}
      {#if hasData}
        <div class="space-y-1">
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            bind:this={canvasContainer}
            class="w-full rounded overflow-hidden nodrag nowheel nopan"
            style="height: 160px; background: #1a1a1a;"
            onpointerdown={(e) => e.stopPropagation()}
            onwheel={(e) => e.stopPropagation()}
          ></div>
          <div class="text-[10px] text-neutral-500 text-right">
            {pointCount.toLocaleString()} points
          </div>
        </div>
      {:else}
        <div class="text-xs text-neutral-500 italic">
          No point cloud yet
        </div>
      {/if}
    {/snippet}
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
