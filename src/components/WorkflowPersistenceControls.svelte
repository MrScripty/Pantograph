<script lang="ts">
  import { FilePlus2, Save, Trash2, X } from 'lucide-svelte';
  import { workflowGraph, isDirty, clearWorkflow } from '../stores/workflowStore';
  import {
    isReadOnly,
    currentGraphId,
    currentGraphName,
    currentGraphType,
    availableWorkflows,
    graphSessionError,
    createNewWorkflow,
    deleteWorkflowByName,
    refreshWorkflowList,
    saveLastGraph,
  } from '../stores/graphSessionStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import GraphSelector from './GraphSelector.svelte';

  const WORKFLOW_FILE_EXTENSION = '.json';

  let isSaving = $state(false);
  let isDeleting = $state(false);
  let actionError = $state<string | null>(null);

  let currentSavedWorkflow = $derived(
    $currentGraphType === 'workflow'
      ? $availableWorkflows.find((workflow) => (workflow.id ?? workflow.name) === $currentGraphId)
      : undefined,
  );

  function normalizeError(error: unknown): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    if (typeof error === 'string' && error.trim().length > 0) {
      return error;
    }
    return String(error);
  }

  function workflowIdFromSavedPath(path: string, fallbackName: string): string {
    const filename = path.split(/[\\/]/).pop();
    if (!filename?.endsWith(WORKFLOW_FILE_EXTENSION)) {
      return fallbackName;
    }
    return filename.slice(0, -WORKFLOW_FILE_EXTENSION.length);
  }

  async function handleSave() {
    if ($isReadOnly || isSaving) return;

    const name = prompt('Workflow name:', $currentGraphName || 'Untitled Workflow')?.trim();
    if (!name) return;

    actionError = null;
    isSaving = true;
    try {
      const path = await workflowService.saveWorkflow(name, $workflowGraph);
      const workflowId = workflowIdFromSavedPath(path, name);

      isDirty.set(false);
      currentGraphId.set(workflowId);
      currentGraphName.set(name);
      saveLastGraph(workflowId, 'workflow');
      await refreshWorkflowList();
    } catch (error) {
      console.error('Failed to save workflow:', error);
      actionError = `Failed to save workflow: ${normalizeError(error)}`;
    } finally {
      isSaving = false;
    }
  }

  function handleNew() {
    if ($isReadOnly) return;
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    actionError = null;
    createNewWorkflow();
  }

  function handleClear() {
    if ($isReadOnly) return;
    if (!confirm('Clear all nodes?')) return;
    actionError = null;
    clearWorkflow();
  }

  async function handleDelete() {
    if ($isReadOnly || isDeleting || !currentSavedWorkflow) return;

    const workflowId = currentSavedWorkflow.id ?? currentSavedWorkflow.name;
    const confirmed = confirm(
      `Delete workflow "${currentSavedWorkflow.name}"? This cannot be undone.`,
    );
    if (!confirmed) return;

    actionError = null;
    isDeleting = true;
    try {
      const deleted = await deleteWorkflowByName(workflowId);
      if (!deleted) {
        actionError = $graphSessionError ?? `Failed to delete workflow "${currentSavedWorkflow.name}".`;
      }
    } catch (error) {
      console.error('Failed to delete workflow:', error);
      actionError = `Failed to delete workflow: ${normalizeError(error)}`;
    } finally {
      isDeleting = false;
    }
  }
</script>

<div class="flex items-center gap-3">
  <GraphSelector />

  <div class="h-6 w-px bg-neutral-700"></div>

  <div class="flex items-center gap-2">
    <button type="button"
      class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors inline-flex items-center gap-1.5"
      class:hover:bg-neutral-700={!$isReadOnly}
      class:opacity-50={$isReadOnly}
      class:cursor-not-allowed={$isReadOnly}
      onclick={handleNew}
      disabled={$isReadOnly}
      title={$isReadOnly ? 'Cannot create new in read-only mode' : 'New Workflow'}
    >
      <FilePlus2 size={14} aria-hidden="true" />
      New
    </button>
    <button type="button"
      class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors inline-flex items-center gap-1.5"
      class:hover:bg-neutral-700={!$isReadOnly && !isSaving}
      class:opacity-50={$isReadOnly || isSaving}
      class:cursor-not-allowed={$isReadOnly || isSaving}
      onclick={handleSave}
      disabled={$isReadOnly || isSaving}
      title={$isReadOnly ? 'Cannot save read-only graph' : 'Save Workflow'}
    >
      <Save size={14} aria-hidden="true" />
      {isSaving ? 'Saving...' : 'Save'}
    </button>
    <button type="button"
      class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors inline-flex items-center gap-1.5"
      class:hover:bg-red-900={!$isReadOnly && currentSavedWorkflow && !isDeleting}
      class:opacity-50={$isReadOnly || !currentSavedWorkflow || isDeleting}
      class:cursor-not-allowed={$isReadOnly || !currentSavedWorkflow || isDeleting}
      onclick={handleDelete}
      disabled={$isReadOnly || !currentSavedWorkflow || isDeleting}
      title={currentSavedWorkflow ? `Delete ${currentSavedWorkflow.name}` : 'Select a saved workflow to delete'}
    >
      <Trash2 size={14} aria-hidden="true" />
      {isDeleting ? 'Deleting...' : 'Delete'}
    </button>
    <button type="button"
      class="px-3 py-1.5 text-sm bg-neutral-800 border border-neutral-600 rounded text-neutral-200 transition-colors inline-flex items-center gap-1.5"
      class:hover:bg-red-900={!$isReadOnly}
      class:opacity-50={$isReadOnly}
      class:cursor-not-allowed={$isReadOnly}
      onclick={handleClear}
      disabled={$isReadOnly}
      title={$isReadOnly ? 'Cannot clear read-only graph' : 'Clear All'}
    >
      <X size={14} aria-hidden="true" />
      Clear
    </button>
  </div>

  {#if actionError}
    <span class="max-w-[240px] truncate text-xs text-red-300" title={actionError}>
      {actionError}
    </span>
  {/if}
</div>
