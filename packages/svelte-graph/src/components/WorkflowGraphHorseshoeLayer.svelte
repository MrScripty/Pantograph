<script lang="ts">
  import type { HorseshoeDragSessionState } from '../horseshoeDragSession.js';
  import {
    resolveHorseshoeSessionStatusLabel,
    type HorseshoeInsertFeedbackState,
  } from '../horseshoeInsertFeedback.js';
  import type { InsertableNodeTypeCandidate } from '../types/workflow.js';
  import HorseshoeDebugOverlay from './HorseshoeDebugOverlay.svelte';
  import HorseshoeInsertSelector from './HorseshoeInsertSelector.svelte';

  interface Props {
    feedback: Pick<HorseshoeInsertFeedbackState, 'pending'>;
    insertableNodeTypes: InsertableNodeTypeCandidate[];
    onCancel: () => void;
    onRotate: (delta: number) => void;
    onSelect: (candidate: InsertableNodeTypeCandidate) => void;
    query: string;
    selectedIndex: number;
    session: HorseshoeDragSessionState;
    trace: string;
  }

  let {
    feedback,
    insertableNodeTypes,
    onCancel,
    onRotate,
    onSelect,
    query,
    selectedIndex,
    session,
    trace,
  }: Props = $props();

  const statusLabel = $derived(
    resolveHorseshoeSessionStatusLabel({
      feedback,
      session,
    }),
  );
</script>

<HorseshoeInsertSelector
  displayState={session.displayState}
  anchorPosition={session.anchorPosition}
  items={insertableNodeTypes}
  {selectedIndex}
  {query}
  pending={feedback.pending}
  {statusLabel}
  {onSelect}
  {onRotate}
  {onCancel}
/>

{#if session.dragActive || session.displayState !== 'hidden' || trace !== 'idle'}
  <HorseshoeDebugOverlay
    {trace}
    displayState={session.displayState}
    blockedReason={session.blockedReason}
  />
{/if}
