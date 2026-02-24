<script lang="ts">
  import { onMount } from 'svelte';
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

  let { id, data, selected = false }: Props = $props();

  // Node color: teal
  const nodeColor = '#14b8a6';

  // Internal segment state
  let segments = $state<PromptSegment[]>(data.segments && data.segments.length > 0
    ? data.segments
    : [{ text: '', masked: false }]);

  // Reference to the contenteditable div
  let editorRef = $state<HTMLDivElement | null>(null);

  // Track whether we need to suppress the next input sync (to avoid cursor jump)
  let suppressSync = $state(false);

  // Persist segments to node data whenever they change
  $effect(() => {
    updateNodeData(id, { segments });
  });

  // Sync segments from external data changes (e.g. undo/redo)
  $effect(() => {
    if (data.segments && data.segments.length > 0) {
      // Only update if meaningfully different
      const incoming = JSON.stringify(data.segments);
      const current = JSON.stringify(segments);
      if (incoming !== current) {
        segments = data.segments;
      }
    }
  });

  /**
   * Render segments into the contenteditable div as styled spans.
   */
  function renderSegments() {
    if (!editorRef) return;

    editorRef.innerHTML = '';
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      const span = document.createElement('span');
      span.dataset.segmentIndex = String(i);
      span.textContent = seg.text;
      if (!seg.masked) {
        // Anchored: green highlight
        span.style.backgroundColor = 'rgba(34, 197, 94, 0.25)';
        span.style.borderRadius = '2px';
        span.style.padding = '0 1px';
      } else {
        // Masked: dimmed
        span.style.opacity = '0.5';
      }
      editorRef.appendChild(span);
    }
  }

  onMount(() => {
    renderSegments();
  });

  // Re-render when segments change (but not during active editing)
  $effect(() => {
    // Trigger on segments array reference change
    const _deps = segments;
    if (!suppressSync) {
      renderSegments();
    }
  });

  /**
   * Parse the contenteditable div back into segments, preserving
   * masked/anchored state based on data-segment-index attributes.
   */
  function parseEditorContent(): PromptSegment[] {
    if (!editorRef) return segments;

    const result: PromptSegment[] = [];
    const childNodes = editorRef.childNodes;

    for (const node of childNodes) {
      let text = '';
      let masked = false;

      if (node.nodeType === Node.TEXT_NODE) {
        // Bare text node (user typed outside a span) — default to not masked
        text = node.textContent || '';
        masked = false;
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        const el = node as HTMLElement;
        text = el.textContent || '';
        const idx = el.dataset.segmentIndex;
        if (idx !== undefined && segments[Number(idx)]) {
          masked = segments[Number(idx)].masked;
        }
      }

      if (text.length > 0) {
        result.push({ text, masked });
      }
    }

    return result.length > 0 ? result : [{ text: '', masked: false }];
  }

  /**
   * Get the current selection range relative to the segments.
   * Returns { startSeg, startOff, endSeg, endOff } or null.
   */
  function getSegmentSelection(): {
    startSeg: number;
    startOffset: number;
    endSeg: number;
    endOffset: number;
  } | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !editorRef) return null;

    const range = sel.getRangeAt(0);

    // Find segment index from a container node
    function findSegIndex(container: globalThis.Node, offset: number): { seg: number; off: number } | null {
      // Walk up to find the span with data-segment-index
      let node: globalThis.Node | null = container;
      while (node && node !== editorRef) {
        if (node.nodeType === Node.ELEMENT_NODE) {
          const el = node as HTMLElement;
          if (el.dataset.segmentIndex !== undefined) {
            return { seg: Number(el.dataset.segmentIndex), off: offset };
          }
        }
        node = node.parentNode;
      }
      // If we're directly in the editor, count through child nodes
      if (container === editorRef) {
        let charCount = 0;
        for (let i = 0; i < editorRef.childNodes.length; i++) {
          const child = editorRef.childNodes[i];
          const len = (child.textContent || '').length;
          if (charCount + len >= offset) {
            const el = child as HTMLElement;
            const idx = el.dataset?.segmentIndex;
            return { seg: idx !== undefined ? Number(idx) : i, off: offset - charCount };
          }
          charCount += len;
        }
      }
      return null;
    }

    const start = findSegIndex(range.startContainer, range.startOffset);
    const end = findSegIndex(range.endContainer, range.endOffset);

    if (!start || !end) return null;

    return {
      startSeg: start.seg,
      startOffset: start.off,
      endSeg: end.seg,
      endOffset: end.off,
    };
  }

  /**
   * Apply mask state to the selected text range.
   * This splits segments at selection boundaries as needed.
   */
  function applyMaskToSelection(masked: boolean) {
    const sel = getSegmentSelection();
    if (!sel) return;

    // First, sync segments from the editor in case user typed
    segments = parseEditorContent();

    const { startSeg, startOffset, endSeg, endOffset } = sel;

    // If selection is collapsed (no range), do nothing
    if (startSeg === endSeg && startOffset === endOffset) return;

    // Build new segment array with the selection range set to the given mask state
    const newSegments: PromptSegment[] = [];

    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];

      if (i < startSeg || i > endSeg) {
        // Outside selection — keep as-is
        newSegments.push({ ...seg });
        continue;
      }

      const isStart = i === startSeg;
      const isEnd = i === endSeg;
      const sOff = isStart ? startOffset : 0;
      const eOff = isEnd ? endOffset : seg.text.length;

      // Part before selection in this segment
      if (sOff > 0) {
        newSegments.push({ text: seg.text.substring(0, sOff), masked: seg.masked });
      }

      // The selected part
      const selectedText = seg.text.substring(sOff, eOff);
      if (selectedText.length > 0) {
        newSegments.push({ text: selectedText, masked });
      }

      // Part after selection in this segment
      if (eOff < seg.text.length) {
        newSegments.push({ text: seg.text.substring(eOff), masked: seg.masked });
      }
    }

    // Merge adjacent segments with the same mask state
    const merged: PromptSegment[] = [];
    for (const seg of newSegments) {
      if (seg.text.length === 0) continue;
      const last = merged[merged.length - 1];
      if (last && last.masked === seg.masked) {
        last.text += seg.text;
      } else {
        merged.push({ ...seg });
      }
    }

    segments = merged.length > 0 ? merged : [{ text: '', masked: false }];
    suppressSync = false;
    renderSegments();
  }

  function handleInput() {
    suppressSync = true;
    segments = parseEditorContent();
    suppressSync = false;
  }

  function handleAnchor() {
    applyMaskToSelection(false);
  }

  function handleMask() {
    applyMaskToSelection(true);
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
          <button
            class="toolbar-btn anchor-btn px-2 py-1 rounded text-xs font-medium transition-colors"
            onclick={handleAnchor}
            title="Mark selection as anchored (preserved during regeneration)"
          >
            Anchor
          </button>
          <button
            class="toolbar-btn mask-btn px-2 py-1 rounded text-xs font-medium transition-colors"
            onclick={handleMask}
            title="Mark selection as masked (regenerated by dLLM)"
          >
            Mask
          </button>
        </div>

        <!-- Contenteditable editor -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          bind:this={editorRef}
          class="editor-area w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1.5 text-sm text-neutral-200 focus:outline-none"
          contenteditable="true"
          oninput={handleInput}
          onkeydown={handleKeyDown}
          role="textbox"
          tabindex="0"
          spellcheck="false"
        ></div>

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
    overflow-y: auto;
    white-space: pre-wrap;
    word-wrap: break-word;
    line-height: 1.5;
  }

  .editor-area:focus {
    border-color: #14b8a6;
  }
</style>
