<script lang="ts">
  import {
    BaseEdge,
    EdgeReconnectAnchor,
    getSmoothStepPath,
    type EdgeProps,
  } from '@xyflow/svelte';

  let {
    id,
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

  const [path, labelX, labelY] = $derived(
    getSmoothStepPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition,
    })
  );

  // Compute style based on selection state
  const computedStyle = $derived(
    selected
      ? 'stroke: #4f46e5; stroke-width: 3px;'
      : (style ?? 'stroke: #525252; stroke-width: 2px;')
  );
</script>

<BaseEdge
  {id}
  {path}
  {labelX}
  {labelY}
  {markerStart}
  {markerEnd}
  {interactionWidth}
  style={computedStyle}
/>
<EdgeReconnectAnchor type="source" position={{ x: sourceX, y: sourceY }} />
<EdgeReconnectAnchor type="target" position={{ x: targetX, y: targetY }} />
