<script lang="ts">
  import { onMount } from 'svelte';
  import { componentRegistry } from '../services/HotLoadRegistry';
  import type { DynamicComponent } from '../types';

  let components: DynamicComponent[] = [];

  onMount(() => {
    const unsubscribe = componentRegistry.subscribe((next) => {
      components = next;
    });
    return () => unsubscribe();
  });
</script>

<div class="fixed inset-0 pointer-events-none z-40">
  {#each components as config (config.id)}
    <div class="pointer-events-auto">
      <svelte:component this={config.component} {...(config.props ?? {})} />
    </div>
  {/each}
</div>
