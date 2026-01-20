<script lang="ts">
  import {
    workflowGraph,
    workflowMetadata,
    isDirty,
    isExecuting,
    setNodeExecutionState,
    resetExecutionStates,
    clearWorkflow,
    loadDefaultWorkflow,
    nodeDefinitions,
  } from '../stores/workflowStore';
  import {
    isReadOnly,
    currentGraphName,
    createNewWorkflow,
  } from '../stores/graphSessionStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { WorkflowEvent } from '../services/workflow/types';
  import { get } from 'svelte/store';
  import GraphSelector from './GraphSelector.svelte';

  let workflowName = $derived($currentGraphName || 'Untitled Workflow');

  async function handleRun() {
    if ($isExecuting) return;

    isExecuting.set(true);
    resetExecutionStates();

    const unsubscribe = workflowService.subscribeEvents(handleWorkflowEvent);

    try {
      await workflowService.executeWorkflow($workflowGraph);
    } catch (error) {
      console.error('Workflow execution failed:', error);
    } finally {
      isExecuting.set(false);
      unsubscribe();
    }
  }

  function handleWorkflowEvent(event: WorkflowEvent) {
    switch (event.type) {
      case 'NodeStarted':
        setNodeExecutionState((event.data as { node_id: string }).node_id, 'running');
        break;
      case 'NodeCompleted':
        setNodeExecutionState((event.data as { node_id: string }).node_id, 'success');
        break;
      case 'NodeError':
        setNodeExecutionState((event.data as { node_id: string }).node_id, 'error');
        break;
    }
  }

  async function handleSave() {
    const name = prompt('Workflow name:', workflowName);
    if (!name) return;

    try {
      await workflowService.saveWorkflow(name, $workflowGraph);
      isDirty.set(false);
    } catch (error) {
      console.error('Failed to save workflow:', error);
    }
  }

  async function handleLoad() {
    // TODO: Open file picker dialog
    console.log('Load workflow');
  }

  function handleNew() {
    if ($isReadOnly) return;
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    createNewWorkflow();
  }

  function handleClear() {
    if ($isReadOnly) return;
    if (!confirm('Clear all nodes?')) return;
    clearWorkflow();
  }
</script>

<div class="workflow-toolbar h-12 px-4 bg-neutral-900 border-b border-neutral-700 flex items-center justify-between">
  <div class="flex items-center gap-3">
    <GraphSelector />

    <div class="h-6 w-px bg-neutral-700"></div>

    <div class="flex items-center gap-2">
      <button
        class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors"
        class:hover:bg-neutral-700={!$isReadOnly}
        class:opacity-50={$isReadOnly}
        class:cursor-not-allowed={$isReadOnly}
        onclick={handleNew}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot create new in read-only mode' : 'New Workflow'}
      >
        [+] New
      </button>
      <button
        class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors"
        class:hover:bg-neutral-700={!$isReadOnly}
        class:opacity-50={$isReadOnly}
        class:cursor-not-allowed={$isReadOnly}
        onclick={handleSave}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot save read-only graph' : 'Save Workflow'}
      >
        [S] Save
      </button>
      <button
        class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors hover:bg-red-900"
        class:opacity-50={$isReadOnly}
        class:cursor-not-allowed={$isReadOnly}
        class:hover:bg-transparent={$isReadOnly}
        onclick={handleClear}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot clear read-only graph' : 'Clear All'}
      >
        [X] Clear
      </button>
    </div>
  </div>

  <div class="flex items-center gap-2">
    {#if $isReadOnly}
      <span class="text-xs text-neutral-500 bg-neutral-800 px-2 py-0.5 rounded">(read-only)</span>
    {/if}
    {#if $isDirty && !$isReadOnly}
      <span class="text-amber-400 text-sm" title="Unsaved changes">*</span>
    {/if}
  </div>

  <div class="flex items-center gap-2">
    <button
      class="px-4 py-1.5 text-sm rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      class:bg-green-600={!$isExecuting}
      class:hover:bg-green-500={!$isExecuting}
      class:bg-amber-600={$isExecuting}
      class:text-white={true}
      onclick={handleRun}
      disabled={$isExecuting}
    >
      {#if $isExecuting}
        [||] Running...
      {:else}
        [>] Run
      {/if}
    </button>
  </div>
</div>
