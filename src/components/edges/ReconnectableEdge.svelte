<script lang="ts">
  import {
    EdgeReconnectAnchor,
    getBezierPath,
    type EdgeProps,
  } from '@xyflow/svelte';
  import { insetReconnectAnchorPosition } from '../reconnectInteraction';
  import {
    INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY,
    INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY,
  } from '../workflowInferenceDriftEdgeOverlays';

  let {
    id,
    data,
    source,
    sourceHandleId,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    interactionWidth = 20,
    selected,
  }: EdgeProps = $props();

  const [path] = $derived(
    getBezierPath({
      sourceX,
      sourceY,
      targetX,
      targetY,
      sourcePosition,
      targetPosition,
    })
  );

  const sourceAnchorPosition = $derived(
    insetReconnectAnchorPosition(
      {
        source,
        sourceHandle: sourceHandleId,
        sourceX,
        sourceY,
        targetX,
        targetY,
      },
      'source',
    )
  );
  const targetAnchorPosition = $derived(
    insetReconnectAnchorPosition(
      {
        source,
        sourceHandle: sourceHandleId,
        sourceX,
        sourceY,
        targetX,
        targetY,
      },
      'target',
    )
  );
  const edgeInsertPreviewActive = $derived(
    (data as Record<string, unknown> | undefined)?.edgeInsertPreviewActive === true,
  );
  const inferenceInterfaceDriftAffected = $derived(
    (data as Record<string, unknown> | undefined)?.[INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY] === true,
  );
  const inferenceInterfaceDriftTitle = $derived(
    (data as Record<string, unknown> | undefined)?.[INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY],
  );
</script>

<!-- Neutral wire (docs/design/mockups/boards/shell.css .wire). The accent is
     kept for live interaction (the insert preview); drift is dashed danger. -->
{#if edgeInsertPreviewActive}
  <path
    class="edge-insert-preview"
    d={path}
    stroke-width={selected ? 9 : 7}
    fill="none"
    stroke-linecap="round"
  />
{/if}

{#if inferenceInterfaceDriftAffected}
  <path
    class="edge-drift"
    d={path}
    stroke-width={selected ? 8 : 6}
    fill="none"
    stroke-linecap="round"
    stroke-dasharray="6 5"
  >
    {#if typeof inferenceInterfaceDriftTitle === 'string'}
      <title>{inferenceInterfaceDriftTitle}</title>
    {/if}
  </path>
{/if}

<path
  {id}
  class="react-flow__edge-path"
  class:edge-selected={selected}
  d={path}
  fill="none"
  stroke-linecap="round"
>
  {#if typeof inferenceInterfaceDriftTitle === 'string'}
    <title>{inferenceInterfaceDriftTitle}</title>
  {/if}
</path>

<!-- Invisible wider path for interaction -->
<path
  d={path}
  stroke="transparent"
  stroke-width={interactionWidth}
  fill="none"
  class="react-flow__edge-interaction"
/>

<EdgeReconnectAnchor type="source" position={sourceAnchorPosition} size={12} />
<EdgeReconnectAnchor type="target" position={targetAnchorPosition} size={12} />

<style>
  .react-flow__edge-path {
    stroke: var(--pg-border-strong);
    stroke-width: 1.5px;
  }

  .react-flow__edge-path.edge-selected {
    stroke: var(--pg-text);
    stroke-width: 2px;
  }

  .edge-insert-preview {
    stroke: var(--pg-accent);
    stroke-opacity: 0.45;
  }

  .edge-drift {
    stroke: var(--pg-danger);
    stroke-opacity: 0.7;
  }
</style>
