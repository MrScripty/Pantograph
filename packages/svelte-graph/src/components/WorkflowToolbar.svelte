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

<div class="workflow-toolbar">
  <div class="toolbar-left">
    {#if children}
      {@render children()}
    {/if}

    <div class="toolbar-divider"></div>

    <div class="toolbar-actions">
      <button
        class="toolbar-btn"
        onclick={handleNew}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot create new in read-only mode' : 'New Workflow'}
      >
        [+] New
      </button>
      <button
        class="toolbar-btn"
        onclick={handleSave}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot save read-only graph' : 'Save Workflow'}
      >
        [S] Save
      </button>
      <button
        class="toolbar-btn toolbar-btn-danger"
        onclick={handleClear}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot clear read-only graph' : 'Clear All'}
      >
        [X] Clear
      </button>
    </div>
  </div>

  <div class="toolbar-center">
    {#if $isReadOnly}
      <span class="readonly-badge">(read-only)</span>
    {/if}
    {#if $isDirty && !$isReadOnly}
      <span class="dirty-indicator" title="Unsaved changes">*</span>
    {/if}
  </div>

  <div class="toolbar-right">
    <button
      class="run-btn"
      data-executing={$isExecuting || undefined}
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

<style>
  .workflow-toolbar {
    height: 3rem;
    padding: 0 1rem;
    background-color: #171717;
    border-bottom: 1px solid #404040;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .toolbar-left,
  .toolbar-center,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .toolbar-left {
    gap: 0.75rem;
  }

  .toolbar-divider {
    height: 1.5rem;
    width: 1px;
    background-color: #404040;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* --- Toolbar Buttons --- */
  .toolbar-btn {
    padding: 0.375rem 0.75rem;
    font-size: 0.875rem;
    background-color: #262626;
    border: 1px solid #525252;
    border-radius: 0.25rem;
    color: #e5e5e5;
    cursor: pointer;
    transition: background-color 150ms;
  }

  .toolbar-btn:not(:disabled):hover {
    background-color: #404040;
  }

  .toolbar-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toolbar-btn-danger:not(:disabled):hover {
    background-color: #7f1d1d;
  }

  /* --- Status indicators --- */
  .readonly-badge {
    font-size: 0.75rem;
    color: #737373;
    background-color: #262626;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
  }

  .dirty-indicator {
    color: #fbbf24;
    font-size: 0.875rem;
  }

  /* --- Run Button --- */
  .run-btn {
    padding: 0.375rem 1rem;
    font-size: 0.875rem;
    border-radius: 0.25rem;
    border: none;
    color: white;
    cursor: pointer;
    transition: background-color 150ms;
    background-color: #16a34a;
  }

  .run-btn:not(:disabled):hover {
    background-color: #22c55e;
  }

  .run-btn[data-executing] {
    background-color: #d97706;
  }

  .run-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
