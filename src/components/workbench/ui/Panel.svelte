<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Accessible name of the region, for example "Runs" or "Run detail". */
    label: string;
    /** Fixed width such as "380px"; omit to fill the remaining space. */
    width?: string;
    /**
     * Column layout (default): fixed `top`, scrolling `children`, fixed
     * `footer`. Set false for a plain bordered box that renders `children`.
     */
    column?: boolean;
    top?: Snippet;
    footer?: Snippet;
    children?: Snippet;
    class?: string;
    /** ARIA role for the scrolling middle, for example "log". */
    scrollRole?: string;
    /** Accessible name for the scrolling middle when it has a role. */
    scrollLabel?: string;
    /** Bindable handle on the scrolling middle, for follow mode and scroll-into-view. */
    scrollElement?: HTMLDivElement | null;
  }

  let {
    label,
    width,
    column = true,
    top,
    footer,
    children,
    class: className = '',
    scrollRole,
    scrollLabel,
    scrollElement = $bindable(null),
  }: Props = $props();
</script>

<section
  class="pg-panel {className}"
  class:pg-col={column}
  aria-label={label}
  style:flex={width ? `0 0 ${width}` : '1 1 0%'}
  style:min-height="0"
>
  {#if column}
    {#if top}
      <div class="pg-ptop">{@render top()}</div>
    {/if}
    <div class="pg-scroll" role={scrollRole} aria-label={scrollLabel} bind:this={scrollElement}>{@render children?.()}</div>
    {#if footer}
      <div class="pg-pfoot">{@render footer()}</div>
    {/if}
  {:else}
    {@render children?.()}
  {/if}
</section>
