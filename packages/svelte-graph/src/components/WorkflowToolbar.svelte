<script lang="ts">
  import { get } from 'svelte/store';
  import { useGraphContext } from '../context/useGraphContext.js';
  import WorkflowRunButton from './WorkflowRunButton.svelte';

  const { backend, stores } = useGraphContext();

  interface Props {
    /** Extension slot (e.g., for a graph selector component) */
    children?: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  const { workflowGraph, isDirty } = stores.workflow;
  const { isReadOnly, currentGraphId, currentGraphName } = stores.session;

  const WORKFLOW_FILE_EXTENSION = '.json';

  let workflowName = $derived($currentGraphName || 'Untitled Workflow');

  function workflowIdFromSavedPath(path: string, fallbackName: string): string {
    const filename = path.split(/[\\/]/).pop();
    if (!filename?.endsWith(WORKFLOW_FILE_EXTENSION)) {
      return fallbackName;
    }
    return filename.slice(0, -WORKFLOW_FILE_EXTENSION.length);
  }

  async function handleSave() {
    const name = prompt('Workflow name:', workflowName);
    if (!name) return;

    try {
      const path = await backend.saveWorkflow(name, get(workflowGraph));
      const workflowId = workflowIdFromSavedPath(path, name);
      isDirty.set(false);

      currentGraphId.set(workflowId);
      currentGraphName.set(name);
      stores.session.saveLastGraph(workflowId, 'workflow');
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
      <button type="button"
        class="toolbar-btn"
        onclick={handleNew}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot create new in read-only mode' : 'New Workflow'}
      >
        [+] New
      </button>
      <button type="button"
        class="toolbar-btn"
        onclick={handleSave}
        disabled={$isReadOnly}
        title={$isReadOnly ? 'Cannot save read-only graph' : 'Save Workflow'}
      >
        [S] Save
      </button>
      <button type="button"
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
    <WorkflowRunButton />
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

</style>
