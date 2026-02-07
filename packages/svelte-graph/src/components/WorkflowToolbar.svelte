<script lang="ts">
  import { get } from 'svelte/store';
  import { useGraphContext } from '../context/useGraphContext.js';
  import type { WorkflowEvent } from '../types/workflow.js';

  const { backend, stores } = useGraphContext();

  interface Props {
    /** Extension slot (e.g., for a graph selector component) */
    children?: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  // Destructure stores for reactive $-prefix access
  const { workflowGraph, isDirty, isExecuting, edges: edgesStore } = stores.workflow;
  const { isReadOnly, currentGraphId, currentGraphName, currentSessionId } = stores.session;

  let workflowName = $derived($currentGraphName || 'Untitled Workflow');

  let currentUnsubscribe: (() => void) | null = null;

  async function handleRun() {
    if ($isExecuting) return;

    isExecuting.set(true);
    stores.workflow.resetExecutionStates();

    currentUnsubscribe = backend.subscribeEvents(handleWorkflowEvent);

    try {
      await backend.executeWorkflow(get(workflowGraph));
    } catch (error) {
      console.error('Workflow execution failed:', error);
      isExecuting.set(false);
      if (currentUnsubscribe) {
        currentUnsubscribe();
        currentUnsubscribe = null;
      }
    }
  }

  function cleanupExecution() {
    isExecuting.set(false);
    if (currentUnsubscribe) {
      currentUnsubscribe();
      currentUnsubscribe = null;
    }
  }

  function handleWorkflowEvent(event: WorkflowEvent) {
    console.log('Workflow event:', event.type, event.data);

    switch (event.type) {
      case 'NodeStarted':
        stores.workflow.setNodeExecutionState((event.data as { node_id: string }).node_id, 'running');
        break;
      case 'NodeCompleted': {
        const completedData = event.data as { node_id: string; outputs?: Record<string, unknown> };
        stores.workflow.setNodeExecutionState(completedData.node_id, 'success');

        if (completedData.outputs) {
          const currentEdges = get(edgesStore);
          const outgoingEdges = currentEdges.filter(e => e.source === completedData.node_id);

          for (const edge of outgoingEdges) {
            const sourceHandle = edge.sourceHandle || '';
            const outputValue = completedData.outputs[sourceHandle];
            if (outputValue !== undefined) {
              const targetHandle = edge.targetHandle || '';
              stores.workflow.updateNodeData(edge.target, {
                [targetHandle]: outputValue,
              });
            }
          }
        }
        break;
      }
      case 'NodeError': {
        const errorData = event.data as { node_id: string; error: string };
        stores.workflow.setNodeExecutionState(errorData.node_id, 'error', errorData.error);
        console.error(`Node ${errorData.node_id} failed:`, errorData.error);
        break;
      }
      case 'Completed':
        console.log('Workflow completed successfully');
        cleanupExecution();
        break;
      case 'Failed': {
        const failedData = event.data as { error: string };
        console.error('Workflow failed:', failedData.error);
        cleanupExecution();
        break;
      }
    }
  }

  async function handleSave() {
    const name = prompt('Workflow name:', workflowName);
    if (!name) return;

    try {
      await backend.saveWorkflow(name, get(workflowGraph));
      isDirty.set(false);

      currentGraphId.set(name);
      currentGraphName.set(name);
      stores.session.saveLastGraph(name, 'workflow');
      await stores.session.refreshWorkflowList();
    } catch (error) {
      console.error('Failed to save workflow:', error);
    }
  }

  function handleNew() {
    if ($isReadOnly) return;
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    stores.session.createNewWorkflow();
  }

  function handleClear() {
    if ($isReadOnly) return;
    if (!confirm('Clear all nodes?')) return;
    stores.workflow.clearWorkflow();
  }
</script>

<div class="workflow-toolbar h-12 px-4 bg-neutral-900 border-b border-neutral-700 flex items-center justify-between">
  <div class="flex items-center gap-3">
    {#if children}
      {@render children()}
    {/if}

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
