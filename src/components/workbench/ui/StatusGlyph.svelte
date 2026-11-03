<script lang="ts">
  import { glyphClass, type GlyphState } from './statusGlyph';

  interface Props {
    state: GlyphState;
    /** 8px instead of 10px (log lines, graph node headers). */
    small?: boolean;
    /** Ink-coloured glyph for use on an accent fill. */
    dark?: boolean;
    class?: string;
  }

  let { state, small = false, dark = false, class: className = '' }: Props = $props();
</script>

<!-- Decorative: callers always render the status in text beside the glyph. -->
<span
  class="pg-g {glyphClass(state)} {className}"
  class:pg-g-sm={small}
  class:pg-g-dark={dark}
  aria-hidden="true"
></span>
