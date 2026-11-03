<script lang="ts">
  import type { Snippet } from 'svelte';
  import StatusGlyph from './StatusGlyph.svelte';
  import type { GlyphState } from './statusGlyph';

  interface Props {
    /** Leading status glyph; the chip text must still name the status. */
    glyph?: GlyphState;
    title?: string;
    children: Snippet;
  }

  let { glyph, title, children }: Props = $props();
</script>

<span class="pg-chip" {title}>
  {#if glyph}<StatusGlyph state={glyph} />{/if}
  {@render children()}
</span>
