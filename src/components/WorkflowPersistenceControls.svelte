<script lang="ts">
  import type { Snippet } from 'svelte';
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
    SYSTEM_GRAPHS,
    switchGraph,
    type GraphType,
  } from '../stores/graphSessionStore';
  import { workflowService } from '../services/workflow/WorkflowService';
  import {
    formatWorkflowModifiedShort,
    type GraphPageMode,
  } from './workbench/graphInspectionPresenters';

  /**
   * Workflow selector (with New, Clear and Delete in its popover) and Save.
   * Renders without a wrapper so its parts sit in the toolbar row; `middle`
   * is rendered between the selector and Save.
   */
  interface Props {
    mode: GraphPageMode;
    /** Version shown next to the workflow name in edit mode. */
    editVersion: string;
    /** In run mode the selector names the run's workflow and version instead. */
    runWorkflow?: { workflowId: string; version: string | null } | null;
    middle?: Snippet;
    /** Called after a picked workflow or system graph has loaded. */
    onworkflowpicked?: () => void;
    /** Save, delete and switch failures, for the toolbar to explain under the row. */
    onactionerror?: (message: string | null) => void;
  }

  let {
    mode,
    editVersion,
    runWorkflow = null,
    middle,
    onworkflowpicked,
    onactionerror,
  }: Props = $props();

  const WORKFLOW_FILE_EXTENSION = '.json';

  let isSaving = $state(false);
  let isDeleting = $state(false);
  let isSwitching = $state(false);
  let open = $state(false);
  let selectorRoot: HTMLDivElement | null = $state(null);
  let toggleButton: HTMLButtonElement | null = $state(null);
  let nowMs = $state(Date.now());

  let currentSavedWorkflow = $derived(
    $currentGraphType === 'workflow'
      ? $availableWorkflows.find((workflow) => (workflow.id ?? workflow.name) === $currentGraphId)
      : undefined,
  );
  let selectorName = $derived(
    mode === 'run' && runWorkflow ? runWorkflow.workflowId : $currentGraphName,
  );
  let selectorVersion = $derived(
    mode === 'run' && runWorkflow
      ? runWorkflow.version
      : $currentGraphType === 'system'
        ? null
        : editVersion,
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

  function reportError(message: string | null): void {
    onactionerror?.(message);
  }

  function workflowIdFromSavedPath(path: string, fallbackName: string): string {
    const filename = path.split(/[\\/]/).pop();
    if (!filename?.endsWith(WORKFLOW_FILE_EXTENSION)) {
      return fallbackName;
    }
    return filename.slice(0, -WORKFLOW_FILE_EXTENSION.length);
  }

  function closePopover(returnFocus: boolean): void {
    open = false;
    if (returnFocus) {
      toggleButton?.focus();
    }
  }

  function togglePopover(): void {
    if (!open) {
      reportError(null);
      nowMs = Date.now();
      void refreshWorkflowList();
    }
    open = !open;
  }

  // Outside pointer and Escape close the popover; listeners live only while it is open.
  $effect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (selectorRoot && !selectorRoot.contains(event.target as Node)) {
        closePopover(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        closePopover(true);
      }
    };
    document.addEventListener('mousedown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('mousedown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  });

  async function handleSelect(id: string, type: GraphType) {
    if (isSwitching) return;

    reportError(null);
    isSwitching = true;
    try {
      const loaded = await switchGraph(id, type);
      if (loaded) {
        closePopover(true);
        onworkflowpicked?.();
        return;
      }
      reportError($graphSessionError ?? `Failed to load ${type} "${id}".`);
    } catch (error) {
      reportError(`Failed to load ${type} "${id}": ${normalizeError(error)}`);
      console.error('[WorkflowPersistenceControls] Failed to switch graph:', error);
    } finally {
      isSwitching = false;
    }
  }

  async function handleSave() {
    if ($isReadOnly || isSaving) return;

    const name = prompt('Workflow name:', $currentGraphName || 'Untitled Workflow')?.trim();
    if (!name) return;

    reportError(null);
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
      reportError(`Failed to save workflow: ${normalizeError(error)}`);
    } finally {
      isSaving = false;
    }
  }

  function handleNew() {
    if ($isReadOnly) return;
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    reportError(null);
    closePopover(true);
    createNewWorkflow();
  }

  function handleClear() {
    if ($isReadOnly) return;
    if (!confirm('Clear all nodes?')) return;
    reportError(null);
    closePopover(true);
    clearWorkflow();
  }

  async function handleDelete() {
    if ($isReadOnly || isDeleting || !currentSavedWorkflow) return;

    const workflowId = currentSavedWorkflow.id ?? currentSavedWorkflow.name;
    const confirmed = confirm(
      `Delete workflow "${currentSavedWorkflow.name}"? This cannot be undone.`,
    );
    if (!confirmed) return;

    reportError(null);
    isDeleting = true;
    try {
      const deleted = await deleteWorkflowByName(workflowId);
      if (!deleted) {
        reportError($graphSessionError ?? `Failed to delete workflow "${currentSavedWorkflow.name}".`);
      }
    } catch (error) {
      console.error('Failed to delete workflow:', error);
      reportError(`Failed to delete workflow: ${normalizeError(error)}`);
    } finally {
      isDeleting = false;
    }
  }
</script>

<div class="wfsel-wrap" bind:this={selectorRoot}>
  <button
    type="button"
    class="wfsel"
    data-testid="workflow-graph-selector-toggle"
    aria-haspopup="dialog"
    aria-expanded={open}
    title="Switch workflow"
    bind:this={toggleButton}
    onclick={togglePopover}
  >
    <b class="wfsel-name">{selectorName}</b>
    {#if selectorVersion}<span class="pg-mono wfsel-ver">v{selectorVersion}</span>{/if}
    {#if mode === 'edit' && $currentGraphType === 'system'}<span class="wfsel-ver">system</span>{/if}
    <span class="wfsel-chev" aria-hidden="true">▾</span>
  </button>

  {#if open}
    <div class="wfpop" role="dialog" aria-label="Workflows">
      <div class="wfhead">Saved workflows</div>
      {#each $availableWorkflows as workflow (workflow.id ?? workflow.name)}
        {@const workflowId = workflow.id ?? workflow.name}
        {@const current = mode === 'edit' && $currentGraphType === 'workflow' && $currentGraphId === workflowId}
        <button
          type="button"
          class="wfrow"
          data-testid="workflow-graph-selector-option"
          data-workflow-id={workflowId}
          aria-current={current ? 'true' : undefined}
          disabled={isSwitching}
          onclick={() => handleSelect(workflowId, 'workflow')}
        >
          <span class="wfrow-name">{workflow.name}</span>
          <span class="pg-mono wfrow-meta">{formatWorkflowModifiedShort(workflow.modified, nowMs)}</span>
        </button>
      {:else}
        <p class="wfnone">No saved workflows yet.</p>
      {/each}

      <div class="wfhead">System graphs</div>
      {#each SYSTEM_GRAPHS as graph (graph.id)}
        {@const current = mode === 'edit' && $currentGraphType === 'system' && $currentGraphId === graph.id}
        <button
          type="button"
          class="wfrow"
          aria-current={current ? 'true' : undefined}
          disabled={isSwitching}
          onclick={() => handleSelect(graph.id, 'system')}
        >
          <span class="wfrow-name">{graph.name}</span>
          <span class="pg-mono wfrow-meta">read-only</span>
        </button>
      {/each}

      {#if mode === 'edit'}
        <div class="wfactions">
          <button
            type="button"
            class="pg-btn pg-btn-sm"
            onclick={handleNew}
            disabled={$isReadOnly}
            title={$isReadOnly ? 'Cannot create new in read-only mode' : 'Start a new, empty workflow'}
          >
            New
          </button>
          <button
            type="button"
            class="pg-btn pg-btn-sm"
            onclick={handleClear}
            disabled={$isReadOnly}
            title={$isReadOnly ? 'Cannot clear read-only graph' : 'Remove every node from the canvas'}
          >
            Clear canvas
          </button>
          <button
            type="button"
            class="pg-btn pg-btn-sm pg-btn-danger wfdelete"
            onclick={handleDelete}
            disabled={$isReadOnly || !currentSavedWorkflow || isDeleting}
            title={currentSavedWorkflow ? `Delete ${currentSavedWorkflow.name}` : 'Select a saved workflow to delete'}
          >
            {isDeleting ? 'Deleting…' : 'Delete'}
          </button>
        </div>
      {/if}
      <div class="wffoot">
        Picking a workflow switches to editing it. To see a workflow as a run used it, pick the run instead.
      </div>
    </div>
  {/if}
</div>

{@render middle?.()}

{#if mode === 'edit'}
  <button
    type="button"
    class="pg-btn"
    onclick={handleSave}
    disabled={$isReadOnly || isSaving}
    title={$isReadOnly ? 'Cannot save read-only graph' : 'Save workflow'}
  >
    {isSaving ? 'Saving…' : 'Save'}
  </button>
{/if}

<style>
  .wfsel-wrap {
    position: relative;
    flex: none;
  }

  .wfsel {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    max-width: 280px;
    padding: 0 10px;
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    background: var(--pg-panel-sunken);
    color: var(--pg-text);
    cursor: pointer;
    font-size: 13px;
  }

  .wfsel:hover {
    border-color: var(--pg-border-strong);
  }

  .wfsel-name {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--pg-text-bright);
  }

  .wfsel-ver {
    font-size: 11.5px;
    color: var(--pg-text-muted);
    white-space: nowrap;
  }

  .wfsel-chev {
    font-size: 10px;
    color: var(--pg-text-muted);
  }

  .wfpop {
    position: absolute;
    left: 0;
    top: 34px;
    z-index: 60;
    width: 320px;
    max-height: 70vh;
    overflow: auto;
    background: var(--pg-panel-sunken);
    border: 1px solid var(--pg-border);
    border-radius: 4px;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
  }

  .wfhead {
    position: sticky;
    top: 0;
    padding: 6px 12px;
    border-top: 1px solid var(--pg-border-soft);
    background: var(--pg-panel-head);
    color: var(--pg-text-soft);
    font-size: 12px;
    font-weight: 700;
  }

  .wfhead:first-child {
    border-top: 0;
  }

  .wfrow {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 12px;
    border: 0;
    border-top: 1px solid var(--pg-border-soft);
    background: transparent;
    color: var(--pg-text);
    text-align: left;
    cursor: pointer;
    font-size: 13px;
  }

  .wfrow:hover {
    background: var(--pg-panel-raised);
  }

  .wfrow:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .wfrow[aria-current='true'] {
    background: var(--pg-panel-selected);
    box-shadow: inset 3px 0 0 var(--pg-text);
  }

  .wfrow-name {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .wfrow-meta {
    margin-left: auto;
    font-size: 11px;
    color: var(--pg-text-muted);
  }

  .wfnone {
    margin: 0;
    padding: 8px 12px;
    font-size: 12.5px;
    color: var(--pg-text-muted);
  }

  .wfactions {
    display: flex;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--pg-border-soft);
  }

  .wfdelete {
    margin-left: auto;
  }

  .wffoot {
    padding: 8px 12px;
    border-top: 1px solid var(--pg-border-soft);
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--pg-border-strong);
  }
</style>
