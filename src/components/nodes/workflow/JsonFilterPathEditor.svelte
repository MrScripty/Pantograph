<script lang="ts">
  import { onDestroy } from 'svelte';
  import { updateNodeData } from '../../../stores/workflowStore';
  import { jsonFilterApplyError, jsonFilterPathState } from './jsonFilterAuthoring';

  interface Props {
    id: string;
    data: { path?: unknown };
    applyNodeData?: typeof updateNodeData;
    invalidData?: boolean;
    inputPrefix?: string;

  }
  let { id, data, applyNodeData = updateNodeData, invalidData = false, inputPrefix = undefined }: Props = $props();
  let savedPath = $derived(invalidData
    ? { text: '', error: 'The saved node data must be an object. Apply a path to repair it.' }
    : jsonFilterPathState(data.path));
  let inputId = $derived(`json-filter-${inputPrefix ?? id}-path`);
  let draftPath = $state('');
  let dirty = $state(false);
  let conflict = $state(false);
  let busy = $state(false);
  let applied = $state(false);
  let error = $state<string | null>(null);
  let observedId: string | null = null;
  let observedPath: unknown;
  let pendingPath: string | null = null;
  let appliedPath: string | null = null;
  let generation = 0;
  let disposed = false;

  $effect(() => { syncSavedPath(id, data.path); });
  onDestroy(() => { disposed = true; generation += 1; });

  function syncSavedPath(nodeId: string, path: unknown): void {
    const state = jsonFilterPathState(path);
    if (nodeId !== observedId) {
      generation += 1;
      observedId = nodeId;
      observedPath = path;
      draftPath = state.text;
      dirty = conflict = busy = applied = false;
      pendingPath = error = null;
      appliedPath = null;
    } else if (path !== observedPath) {
      observedPath = path;
      if (busy) {
        // An own response can arrive before the await resumes. A subsequent
        // undo/other edit must still conflict with this pending submission.
        dirty = conflict = path !== pendingPath;
        applied = false;
      } else if (dirty) {
        // Keep an unapplied draft, but require an explicit reload after a
        // different saved path arrives. Do not silently overwrite either value.
        conflict = state.error !== null || draftPath !== state.text;
        applied = false;
      } else {
        draftPath = state.text;
        conflict = false;
        applied = applied && path === appliedPath;
        error = null;
      }
    }
  }

  function handleInput(event: Event): void {
    if (busy) return;
    draftPath = (event.currentTarget as HTMLTextAreaElement).value;
    dirty = savedPath.error !== null || draftPath !== savedPath.text;
    applied = false;
    error = null;
  }

  function reloadSavedPath(): void {
    if (busy) return;
    draftPath = savedPath.text;
    dirty = conflict = applied = false;
    error = null;
  }

  async function applyPath(): Promise<void> {
    if (busy || conflict || (!dirty && savedPath.error === null)) return;
    const attempt = ++generation;
    const nodeId = id;
    const path = draftPath;
    busy = true;
    pendingPath = path;
    applied = false;
    error = null;
    try {
      const result = await applyNodeData(nodeId, { path });
      if (disposed || generation !== attempt || id !== nodeId) return;
      error = jsonFilterApplyError(result);
      if (error === null && !conflict) {
        dirty = false;
        appliedPath = path;
        applied = true;
      }
    } catch (failure) {
      if (!disposed && generation === attempt && id === nodeId) error = String(failure);
    } finally {
      if (!disposed && generation === attempt && id === nodeId) {
        busy = false;
        pendingPath = null;
      }
    }
  }
</script>


  <div class="space-y-2">
    <label for={inputId} class="block text-xs text-neutral-300">JSON path</label>
    <textarea
      id={inputId}
      class="nodrag nopan nowheel w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-xs text-neutral-200"
      rows="2"
      value={draftPath}
      placeholder="results[0].document.text"
      disabled={busy}
      aria-describedby={`${inputId}-help`}
      oninput={handleInput}
    ></textarea>
    <p id={`${inputId}-help`} class="text-[10px] text-neutral-400">
      Use field.subfield or items[0].field. An empty path selects the whole JSON value.
      The value and found outputs come from the backend.
    </p>
    {#if savedPath.error}<p role="alert" class="text-xs text-red-300">{savedPath.error}</p>{/if}
    {#if conflict}
      <p role="alert" class="text-xs text-amber-300">The saved path changed while you were editing. Your draft is retained; reload the saved path before applying.</p>
    {/if}
    <div class="flex gap-2">
      <button type="button" class="nodrag nopan nowheel rounded bg-neutral-700 px-2 py-1 text-xs disabled:opacity-50"
        disabled={busy || conflict || (!dirty && savedPath.error === null)} onclick={applyPath}>
        {busy ? 'Applying…' : 'Apply path'}
      </button>
      <button type="button" class="nodrag nopan nowheel rounded px-2 py-1 text-xs text-neutral-300 disabled:opacity-50"
        disabled={busy || (!dirty && !conflict)} onclick={reloadSavedPath}>Reload saved path</button>
    </div>
    {#if error}<p role="alert" class="text-xs text-red-300">{error}</p>{/if}
    {#if applied}<p role="status" class="text-xs text-green-300">Path applied to the graph. Save the workflow to retain it.</p>{/if}
  </div>
