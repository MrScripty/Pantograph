<script lang="ts">
  import StatusGlyph from './StatusGlyph.svelte';
  import type { GlyphState } from './statusGlyph';

  interface Props {
    /** Status glyph; omit for rows that carry no status. */
    glyph?: GlyphState;
    name: string;
    /** Mono suffix after the name, for example "v1.4.0". */
    version?: string | null;
    /** First sub-line: status word, rendered in danger colour when `statusDanger`. */
    statusLabel?: string;
    statusDanger?: boolean;
    /** Text after the status word, for example ", #2 in queue". */
    context?: string;
    /** Second, smaller sub-line, for example the placement. */
    detail?: string;
    /** Mono text on the right, for example "20:05:51". */
    time?: string;
    selected?: boolean;
    /** Makes Ctrl+C over the row copy this run id (see shell/runContextClipboard.ts). */
    runId?: string;
    title?: string;
    id?: string;
    onselect: () => void;
  }

  let {
    glyph,
    name,
    version = null,
    statusLabel,
    statusDanger = false,
    context = '',
    detail,
    time,
    selected = false,
    runId,
    title,
    id,
    onselect,
  }: Props = $props();

  let rowTitle = $derived(title ?? (runId ? `${runId}. Ctrl+C copies the run id` : undefined));
</script>

<button
  type="button"
  class="pg-item"
  {id}
  aria-current={selected ? 'true' : undefined}
  data-run-id={runId}
  title={rowTitle}
  onclick={onselect}
>
  {#if glyph}<StatusGlyph state={glyph} />{:else}<span class="pg-g-spacer" aria-hidden="true"></span>{/if}
  <span class="min-w-0">
    <span class="pg-name">{name}</span>{#if version} <span class="pg-ver">{version}</span>{/if}
    {#if statusLabel || context}
      <span class="pg-sub"><span class:pg-st-failed={statusDanger}>{statusLabel ?? ''}</span>{context}</span>
    {/if}
    {#if detail}
      <span class="pg-sub pg-detail">{detail}</span>
    {/if}
  </span>
  <span class="pg-time">{time ?? ''}</span>
</button>

<style>
  .pg-g-spacer {
    display: inline-block;
    width: 10px;
    height: 10px;
  }

  .pg-detail {
    font-size: 12px;
  }
</style>
