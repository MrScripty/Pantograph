<script lang="ts">
  import type { EnvironmentRef } from './dependencyEnvironmentState';

  interface Props {
    environmentRef: EnvironmentRef;
  }

  let { environmentRef }: Props = $props();
</script>

<div class="rounded border border-pg-border bg-pg-panel-sunken px-2 py-1 text-[10px] space-y-1">
  <div class="text-pg-text">environment state: {environmentRef.state}</div>
  {#if environmentRef.env_id}
    <div class="text-pg-text-soft truncate" title={environmentRef.env_id}>
      env: {environmentRef.env_id}
    </div>
  {/if}
  {#if environmentRef.python_executable}
    <div class="text-pg-text-muted truncate" title={environmentRef.python_executable}>
      python: {environmentRef.python_executable}
    </div>
  {/if}
</div>
