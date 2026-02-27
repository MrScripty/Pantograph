<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';

  interface PromptSegment {
    text: string;
    masked: boolean;
  }

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      segments?: PromptSegment[];
    };
    selected?: boolean;
  }

  const EMPTY_SEGMENT: PromptSegment = { text: '', masked: false };

  function normalizeSegments(input: PromptSegment[]): PromptSegment[] {
    const merged: PromptSegment[] = [];
    for (const seg of input) {
      if (!seg.text) continue;
      const last = merged[merged.length - 1];
      if (last && last.masked === seg.masked) {
        last.text += seg.text;
      } else {
        merged.push({ text: seg.text, masked: seg.masked });
      }
    }
    return merged.length > 0 ? merged : [{ ...EMPTY_SEGMENT }];
  }

  function segmentsToText(input: PromptSegment[]): string {
    return input.map((seg) => seg.text).join('');
  }

  function expandSegmentsToFlags(text: string, input: PromptSegment[]): boolean[] {
    const flags: boolean[] = [];
    for (const seg of input) {
      for (let i = 0; i < seg.text.length; i++) {
        flags.push(seg.masked);
      }
    }

    if (flags.length > text.length) {
      flags.length = text.length;
    }
    while (flags.length < text.length) {
      flags.push(false);
    }
    return flags;
  }

  function segmentsFromTextAndFlags(text: string, flags: boolean[]): PromptSegment[] {
    if (text.length === 0) return [{ ...EMPTY_SEGMENT }];

    const segments: PromptSegment[] = [];
    let currentMasked = flags[0] ?? false;
    let currentText = text[0];

    for (let i = 1; i < text.length; i++) {
      const nextMasked = flags[i] ?? false;
      if (nextMasked === currentMasked) {
        currentText += text[i];
      } else {
        segments.push({ text: currentText, masked: currentMasked });
        currentMasked = nextMasked;
        currentText = text[i];
      }
    }

    segments.push({ text: currentText, masked: currentMasked });
    return normalizeSegments(segments);
  }

  function applyTextEditToSegments(
    oldText: string,
    newText: string,
    currentSegments: PromptSegment[]
  ): PromptSegment[] {
    const oldFlags = expandSegmentsToFlags(oldText, currentSegments);

    let prefix = 0;
    while (
      prefix < oldText.length &&
      prefix < newText.length &&
      oldText[prefix] === newText[prefix]
    ) {
      prefix++;
    }

    let oldSuffixStart = oldText.length;
    let newSuffixStart = newText.length;
    while (
      oldSuffixStart > prefix &&
      newSuffixStart > prefix &&
      oldText[oldSuffixStart - 1] === newText[newSuffixStart - 1]
    ) {
      oldSuffixStart--;
      newSuffixStart--;
    }

    const nextFlags: boolean[] = [];
    for (let i = 0; i < prefix; i++) {
      nextFlags.push(oldFlags[i] ?? false);
    }

    const insertedLength = newSuffixStart - prefix;
    const inheritedMask =
      (prefix > 0 ? oldFlags[prefix - 1] : oldFlags[oldSuffixStart]) ?? false;
    for (let i = 0; i < insertedLength; i++) {
      nextFlags.push(inheritedMask);
    }

    for (let i = oldSuffixStart; i < oldText.length; i++) {
      nextFlags.push(oldFlags[i] ?? false);
    }

    return segmentsFromTextAndFlags(newText, nextFlags);
  }

  let { id, data, selected = false }: Props = $props();

  const nodeColor = '#14b8a6';
  const segmentsFromData = () => normalizeSegments(
    data.segments && data.segments.length > 0
      ? data.segments
      : [{ ...EMPTY_SEGMENT }],
  );

  let segments = $state<PromptSegment[]>(segmentsFromData());
  let editorText = $state(segmentsToText(segmentsFromData()));
  let editorRef = $state<HTMLTextAreaElement | null>(null);
  let cachedSelection = $state({ start: 0, end: 0 });

  // Persist segments to node data whenever they change
  $effect(() => {
    updateNodeData(id, { segments });
  });

  // Sync from external data changes (e.g. undo/redo)
  $effect(() => {
    const incoming = normalizeSegments(data.segments && data.segments.length > 0
      ? data.segments
      : [{ ...EMPTY_SEGMENT }]);

    if (JSON.stringify(incoming) !== JSON.stringify(segments)) {
      segments = incoming;
      editorText = segmentsToText(incoming);
    }
  });

  function cacheSelection() {
    if (!editorRef) return;
    cachedSelection = {
      start: editorRef.selectionStart ?? 0,
      end: editorRef.selectionEnd ?? 0,
    };
  }

  function applyMaskToSelection(masked: boolean) {
    const start = editorRef?.selectionStart ?? cachedSelection.start;
    const end = editorRef?.selectionEnd ?? cachedSelection.end;
    const from = Math.max(0, Math.min(start, end));
    const to = Math.min(editorText.length, Math.max(start, end));
    if (from === to) return;

    const flags = expandSegmentsToFlags(editorText, segments);
    for (let i = from; i < to; i++) {
      flags[i] = masked;
    }

    segments = segmentsFromTextAndFlags(editorText, flags);

    if (editorRef) {
      editorRef.focus();
      editorRef.setSelectionRange(from, to);
      cacheSelection();
    }
  }

  function handleInput(e: Event) {
    const target = e.currentTarget as HTMLTextAreaElement;
    const nextText = target.value;
    const prevText = editorText;

    if (nextText === prevText) return;

    segments = applyTextEditToSegments(prevText, nextText, segments);
    editorText = nextText;
    cacheSelection();
  }

  function handleAnchor() {
    applyMaskToSelection(false);
  }

  function handleMask() {
    applyMaskToSelection(true);
  }

  function preserveSelection(e: MouseEvent) {
    e.preventDefault();
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Prevent the node from being deleted when pressing Delete/Backspace inside the editor
    e.stopPropagation();
  }
</script>

<div class="masked-text-input-wrapper" style="--node-color: {nodeColor}">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div
          class="w-5 h-5 rounded flex items-center justify-center flex-shrink-0"
          style="background-color: {nodeColor}"
        >
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Masked Text Input'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="masked-text-content">
        <!-- Toolbar -->
        <div class="toolbar flex gap-1 mb-2">
          <button type="button"
            class="toolbar-btn anchor-btn px-2 py-1 rounded text-xs font-medium transition-colors"
            onmousedown={preserveSelection}
            onclick={handleAnchor}
            title="Mark selection as anchored (preserved during regeneration)"
          >
            Anchor
          </button>
          <button type="button"
            class="toolbar-btn mask-btn px-2 py-1 rounded text-xs font-medium transition-colors"
            onmousedown={preserveSelection}
            onclick={handleMask}
            title="Mark selection as masked (regenerated by dLLM)"
          >
            Mask
          </button>
        </div>

        <textarea
          bind:this={editorRef}
          class="editor-area w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1.5 text-sm text-neutral-200 focus:outline-none"
          value={editorText}
          oninput={handleInput}
          onselect={cacheSelection}
          onmouseup={cacheSelection}
          onkeyup={cacheSelection}
          onkeydown={handleKeyDown}
          aria-label="Masked text editor"
          spellcheck="false"
        ></textarea>

        <div class="segment-preview mt-1.5">
          {#if editorText.length === 0}
            <span class="text-[10px] text-neutral-600 italic">No segments yet</span>
          {:else}
            {#each segments as seg, index (`${index}-${seg.masked}-${seg.text}`)}
              {#if seg.text.length > 0}
                <span
                  class="segment-chip"
                  class:segment-chip--anchored={!seg.masked}
                  class:segment-chip--masked={seg.masked}
                  title={seg.masked ? 'Masked segment' : 'Anchored segment'}
                >
                  {seg.text}
                </span>
              {/if}
            {/each}
          {/if}
        </div>

        <!-- Legend -->
        <div class="flex items-center gap-3 mt-1.5 text-[10px] text-neutral-500">
          <div class="flex items-center gap-1">
            <span class="inline-block w-2.5 h-2.5 rounded-sm" style="background-color: rgba(34, 197, 94, 0.25); border: 1px solid rgba(34, 197, 94, 0.4);"></span>
            <span>Anchored</span>
          </div>
          <div class="flex items-center gap-1">
            <span class="inline-block w-2.5 h-2.5 rounded-sm" style="background-color: rgba(163, 163, 163, 0.15); border: 1px solid rgba(163, 163, 163, 0.3);"></span>
            <span>Masked</span>
          </div>
        </div>
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .masked-text-input-wrapper :global(.base-node) {
    border-color: color-mix(in srgb, var(--node-color) 50%, transparent);
  }

  .masked-text-input-wrapper :global(.node-header) {
    background-color: color-mix(in srgb, var(--node-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--node-color) 30%, transparent);
  }

  .toolbar-btn {
    background-color: #262626;
    border: 1px solid #404040;
    color: #a3a3a3;
    cursor: pointer;
  }

  .toolbar-btn:hover {
    background-color: #404040;
    color: #e5e5e5;
  }

  .anchor-btn:hover {
    border-color: #22c55e;
    color: #22c55e;
  }

  .mask-btn:hover {
    border-color: #a3a3a3;
    color: #a3a3a3;
  }

  .editor-area {
    min-height: 3rem;
    max-height: 8rem;
    width: 100%;
    overflow-y: auto;
    resize: vertical;
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.5;
    font-family: inherit;
  }

  .editor-area:focus {
    border-color: #14b8a6;
  }

  .segment-preview {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    white-space: pre-wrap;
  }

  .segment-chip {
    display: inline-block;
    font-size: 0.625rem;
    line-height: 1.3;
    border-radius: 0.25rem;
    padding: 0.125rem 0.375rem;
    max-width: 100%;
    word-break: break-word;
  }

  .segment-chip--anchored {
    color: #86efac;
    border: 1px solid rgba(34, 197, 94, 0.4);
    background: rgba(34, 197, 94, 0.2);
  }

  .segment-chip--masked {
    color: #d4d4d4;
    border: 1px solid rgba(163, 163, 163, 0.35);
    background: rgba(163, 163, 163, 0.15);
  }
</style>
