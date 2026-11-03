<script lang="ts">
  import type { KeyValueRow } from './types';

  interface Props {
    rows: KeyValueRow[];
    /** Accessible name when the list has no visible heading nearby. */
    label?: string;
  }

  let { rows, label }: Props = $props();
</script>

<dl class="pg-kv" aria-label={label}>
  {#each rows as row (row.key)}
    <dt>{row.key}</dt>
    <dd class:pg-mono={row.mono} data-run-id={row.runId}>{row.value}</dd>
  {/each}
</dl>
