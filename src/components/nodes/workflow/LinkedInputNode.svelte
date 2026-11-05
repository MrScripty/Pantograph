<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';
  import {
    linkMappings,
    linkModeActive,
    startLinkMode,
    unlinkNode,
    clearNodeLink,
  } from '../../../stores/linkStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      linked_value?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();


  // Get the link mapping for this node
  let mapping = $derived($linkMappings.get(id));
  let status = $derived(mapping?.status ?? 'unlinked');
  let linkedValue = $derived(mapping?.lastValue ?? '');
  let elementLabel = $derived(mapping?.elementLabel ?? '');
  let errorMessage = $derived(mapping?.errorMessage ?? '');

  // Sync linked value to node data for execution
  $effect(() => {
    if (status === 'linked' && linkedValue !== data.linked_value) {
      updateNodeData(id, { linked_value: linkedValue });
    }
  });

  function handleLink() {
    startLinkMode(id);
  }

  function handleUnlink() {
    unlinkNode(id);
    updateNodeData(id, { linked_value: undefined });
  }

  function handleClear() {
    clearNodeLink(id);
    updateNodeData(id, { linked_value: undefined });
  }

  function handleRelink() {
    startLinkMode(id);
  }

  // Truncate value for display
  function truncateValue(value: string, maxLength: number = 30): string {
    if (value.length <= maxLength) return value;
    return value.substring(0, maxLength) + '...';
  }
</script>

<div class="linked-input-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Linked Input'} />
    {/snippet}

      <div class="linked-input-content">
        {#if status === 'unlinked'}
          <!-- Unlinked state -->
          <button type="button"
            class="link-button w-full px-3 py-2 border border-pg-border bg-pg-panel-raised hover:bg-pg-panel-selected rounded text-sm text-pg-text-soft flex items-center justify-center gap-2 transition-colors"
            onclick={handleLink}
            disabled={$linkModeActive}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
              />
            </svg>
            Link to Element
          </button>
        {:else if status === 'linked'}
          <!-- Linked state -->
          <div class="linked-state">
            <div class="flex items-center gap-1.5 text-pg-ok text-xs mb-1">
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
              <span>Linked to: {elementLabel}</span>
            </div>
            <div class="value-preview text-xs text-pg-text-muted bg-pg-panel-sunken rounded px-2 py-1 mb-2 font-mono">
              {truncateValue(linkedValue) || '(empty)'}
            </div>
            <div class="flex gap-1">
              <button type="button"
                class="flex-1 px-2 py-1 border border-pg-border bg-pg-panel-raised hover:bg-pg-panel-selected rounded text-xs text-pg-text-soft transition-colors"
                onclick={handleUnlink}
              >
                Unlink
              </button>
              <button type="button"
                class="flex-1 px-2 py-1 border border-pg-border bg-pg-panel-raised hover:bg-pg-panel-selected rounded text-xs text-pg-text-soft transition-colors"
                onclick={handleRelink}
                disabled={$linkModeActive}
              >
                Re-link
              </button>
            </div>
          </div>
        {:else if status === 'error'}
          <!-- Error state -->
          <div class="error-state">
            <div class="flex items-center gap-1.5 text-pg-text-soft text-xs mb-1">
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
              <span>{errorMessage}</span>
            </div>
            <div class="text-xs text-pg-text-muted mb-2">
              Previously: {elementLabel}
            </div>
            <div class="flex gap-1">
              <button type="button"
                class="flex-1 px-2 py-1 border border-pg-border bg-pg-panel-raised hover:bg-pg-panel-selected rounded text-xs text-pg-text-soft transition-colors"
                onclick={handleRelink}
                disabled={$linkModeActive}
              >
                Re-link
              </button>
              <button type="button"
                class="flex-1 px-2 py-1 border border-pg-border bg-pg-panel-raised hover:bg-pg-panel-selected rounded text-xs text-pg-text-soft transition-colors"
                onclick={handleClear}
              >
                Clear
              </button>
            </div>
          </div>
        {/if}
      </div>
  </BaseNode>
</div>

<style>
  .link-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .value-preview {
    max-height: 3rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
