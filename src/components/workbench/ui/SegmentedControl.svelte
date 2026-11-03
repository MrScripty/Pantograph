<script lang="ts" generics="T extends string">
  import type { SegmentOption } from './types';

  interface Props {
    /** Accessible name of the group, for example "Status". */
    label: string;
    options: SegmentOption<T>[];
    value: T;
    onchange: (value: T) => void;
    /** Size buttons to their content instead of sharing the width equally. */
    compact?: boolean;
    class?: string;
  }

  let { label, options, value, onchange, compact = false, class: className = '' }: Props = $props();
</script>

<div class="pg-seg {className}" class:pg-seg-compact={compact} role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      aria-pressed={option.value === value}
      disabled={option.disabled}
      title={option.title}
      onclick={() => onchange(option.value)}
    >
      {option.label}
      {#if option.count !== undefined}<span class="pg-n">{option.count}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .pg-seg-compact {
    flex: none;
  }

  .pg-seg-compact button {
    flex: none;
    padding: 0 10px;
  }
</style>
