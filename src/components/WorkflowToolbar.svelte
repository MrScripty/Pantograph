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
  import { workflowService } from '../services/workflow/WorkflowService';
  import type { WorkflowEvent } from '../services/workflow/types';
  import { get } from 'svelte/store';

  let workflowName = $derived($workflowMetadata?.name || 'Untitled Workflow');

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
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    loadDefaultWorkflow(get(nodeDefinitions));
  }

  function handleClear() {
    if (!confirm('Clear all nodes?')) return;
    clearWorkflow();
  }
</script>

<div class="workflow-toolbar h-12 px-4 bg-neutral-900 border-b border-neutral-700 flex items-center justify-between">
  <div class="flex items-center gap-2">
    <button
      class="px-3 py-1.5 text-sm bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded text-neutral-200 transition-colors"
      onclick={handleNew}
      title="New Workflow"
    >
      [+] New
    </button>
    <button
      class="px-3 py-1.5 text-sm bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded text-neutral-200 transition-colors"
      onclick={handleSave}
      title="Save Workflow"
    >
      [S] Save
    </button>
    <button
      class="px-3 py-1.5 text-sm bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded text-neutral-200 transition-colors"
      onclick={handleLoad}
      title="Load Workflow"
    >
      [L] Load
    </button>
    <button
      class="px-3 py-1.5 text-sm bg-neutral-800 hover:bg-red-900/50 border border-neutral-600 rounded text-neutral-200 transition-colors"
      onclick={handleClear}
      title="Clear All"
    >
      [X] Clear
    </button>
  </div>

  <div class="flex items-center gap-2">
    <span class="text-sm text-neutral-300">
      {workflowName}
    </span>
    {#if $isDirty}
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
