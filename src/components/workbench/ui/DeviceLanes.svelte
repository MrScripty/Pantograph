<script lang="ts">
  import StatusGlyph from './StatusGlyph.svelte';
  import type { GlyphState } from './statusGlyph';
  import type { DeviceLane, DeviceLaneBlockKind } from './types';

  interface Props {
    lanes: DeviceLane[];
    /** Short scope label on the left, for example "local" or a node name. */
    scopeLabel?: string;
    /** Accessible name of the strip. */
    label?: string;
    showLegend?: boolean;
  }

  let { lanes, scopeLabel, label = 'Device memory', showLegend = true }: Props = $props();

  const BLOCK_CLASS: Record<DeviceLaneBlockKind, string> = {
    running: 'pg-blk-running',
    waiting: 'pg-blk-waiting',
    queued: 'pg-blk-queued',
    free: 'pg-blk-free',
  };

  const BLOCK_GLYPH: Record<DeviceLaneBlockKind, GlyphState | null> = {
    running: 'running',
    waiting: 'waiting',
    queued: 'queued',
    free: null,
  };
</script>

<section class="pg-lanes" aria-label={label}>
  {#if scopeLabel}
    <div class="pg-mono pg-muted pg-lanes-scope">{scopeLabel}</div>
  {/if}
  <div class="pg-lanes-grid">
    {#each lanes as lane (lane.id)}
      <div class="pg-lane">
        <div class="pg-lane-name">
          {lane.name}<span class="pg-muted pg-lane-meta">{lane.meta}</span>
        </div>
        <div class="pg-lane-track" title={lane.meta}>
          {#each lane.blocks as block (block.id)}
            {@const glyph = BLOCK_GLYPH[block.kind]}
            <div class="pg-blk {BLOCK_CLASS[block.kind]}" style:flex="{Math.max(block.gb, 0)} 1 0" title={block.title}>
              {#if glyph}<StatusGlyph state={glyph} small dark={block.kind === 'running'} />{/if}{block.label}
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
  {#if showLegend}
    <div class="pg-legend">
      <span><StatusGlyph state="running" /> running</span>
      <span><StatusGlyph state="waiting" /> waiting for memory</span>
      <span><StatusGlyph state="queued" /> next</span>
    </div>
  {/if}
</section>

<style>
  .pg-lanes-scope {
    font-size: 11px;
    flex: none;
  }

  .pg-lanes-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(320px, 100%), 1fr));
    gap: 8px 22px;
    min-width: 0;
  }

  .pg-lane-meta {
    display: block;
    font-size: 10.5px;
  }

  .pg-legend span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
</style>
