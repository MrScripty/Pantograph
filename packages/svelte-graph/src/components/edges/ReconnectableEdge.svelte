<script lang="ts">
  import {
    EdgeReconnectAnchor,
    getBezierPath,
    type EdgeProps,
  } from '@xyflow/svelte';
  import { useGraphContext } from '../../context/useGraphContext.js';
  import type { NodeDefinition } from '../../types/workflow.js';

  const { stores } = useGraphContext();
  const nodesStore = stores.workflow.nodes;

  let {
    id,
    source,
    target,
    sourceHandleId,
    targetHandleId,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    style,
    markerEnd,
    markerStart,
    interactionWidth = 20,
    selected,
  }: EdgeProps = $props();

  // Port type colors - brighter/more saturated versions for edges
  const typeColors: Record<string, string> = {
    string: '#4ade80',
    prompt: '#60a5fa',
    number: '#fbbf24',
    boolean: '#f87171',
    image: '#a78bfa',
    audio: '#f9a8d4',
    stream: '#22d3ee',
    json: '#fb923c',
    component: '#f472b6',
    document: '#2dd4bf',
    tools: '#fbbf24',
    embedding: '#818cf8',
    vector_db: '#c084fc',
    any: '#9ca3af',
  };

  // Get source port color
  const sourceColor = $derived.by(() => {
    const sourceNode = $nodesStore.find(n => n.id === source);
    if (!sourceNode?.data?.definition) return '#60a5fa';
    const def = sourceNode.data.definition as NodeDefinition;
    const port = def.outputs.find(p => p.id === sourceHandleId);
    return port ? (typeColors[port.data_type] || '#60a5fa') : '#60a5fa';
  });

  // Get target port color
  const targetColor = $derived.by(() => {
    const targetNode = $nodesStore.find(n => n.id === target);
    if (!targetNode?.data?.definition) return '#60a5fa';
    const def = targetNode.data.definition as NodeDefinition;
    const port = def.inputs.find(p => p.id === targetHandleId);
    return port ? (typeColors[port.data_type] || '#60a5fa') : '#60a5fa';
  });

  const [path, labelX, labelY] = $derived(
    getBezierPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition,
    })
  );

  // Unique gradient ID for this edge
  const gradientId = $derived(`edge-gradient-${id}`);
</script>

<!-- Custom SVG with gradient - colored only at the ends, white in the middle -->
<svg style="position: absolute; width: 0; height: 0; overflow: visible;">
  <defs>
    <linearGradient id={gradientId} gradientUnits="userSpaceOnUse" x1={sourceX} y1={sourceY} x2={targetX} y2={targetY}>
      <stop offset="0%" stop-color={sourceColor} />
      <stop offset="12%" stop-color="#ffffff" />
      <stop offset="88%" stop-color="#ffffff" />
      <stop offset="100%" stop-color={targetColor} />
    </linearGradient>
    <filter id="{gradientId}-glow" x="-100%" y="-100%" width="300%" height="300%">
      <feGaussianBlur in="SourceGraphic" stdDeviation="2" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
  </defs>
</svg>

<!-- Main edge path with gradient stroke and glow filter -->
<path
  {id}
  class="react-flow__edge-path"
  d={path}
  stroke="url(#{gradientId})"
  stroke-width={selected ? 1.5 : 1}
  fill="none"
  stroke-linecap="round"
  filter="url(#{gradientId}-glow)"
/>

<!-- Invisible wider path for interaction -->
<path
  d={path}
  stroke="transparent"
  stroke-width={interactionWidth}
  fill="none"
  class="react-flow__edge-interaction"
/>

<EdgeReconnectAnchor type="source" position={{ x: sourceX, y: sourceY }} />
<EdgeReconnectAnchor type="target" position={{ x: targetX, y: targetY }} />
