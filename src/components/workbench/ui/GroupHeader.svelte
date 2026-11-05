<script lang="ts">
  interface Props {
    label: string;
    count?: number | string;
    expanded: boolean;
    ontoggle: () => void;
    /** id of the element holding the group's rows, when there is one. */
    controls?: string;
  }

  let { label, count, expanded, ontoggle, controls }: Props = $props();
</script>

<button
  type="button"
  class="pg-ghead"
  aria-expanded={expanded}
  aria-controls={controls}
  onclick={ontoggle}
>
  {label}
  {#if count !== undefined}<span class="pg-n">{count}</span>{/if}
  <span class="pg-chev" aria-hidden="true">{expanded ? '▾' : '▸'}</span>
</button>
