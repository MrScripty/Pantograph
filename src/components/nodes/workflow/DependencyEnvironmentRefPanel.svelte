<script lang="ts">
  import type { EnvironmentRef } from './dependencyEnvironmentState';

  interface Props {
    environmentRef: EnvironmentRef;
  }

  let { environmentRef }: Props = $props();
</script>

<div class="rounded border border-cyan-900/50 bg-cyan-950/20 px-2 py-1 text-[10px] space-y-1">
  <div class="text-cyan-300">environment state: {environmentRef.state}</div>
  {#if environmentRef.env_id}
    <div class="text-neutral-300 truncate" title={environmentRef.env_id}>
      env: {environmentRef.env_id}
    </div>
  {/if}
  {#if environmentRef.python_executable}
    <div class="text-neutral-400 truncate" title={environmentRef.python_executable}>
      python: {environmentRef.python_executable}
    </div>
  {/if}
</div>
