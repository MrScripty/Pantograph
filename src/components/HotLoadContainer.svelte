<script lang="ts">
  import { onMount } from 'svelte';
  import { componentRegistry, type GeneratedComponent } from '../services/HotLoadRegistry';
  import { panelWidth } from '../stores/panelStore';

  let components: GeneratedComponent[] = [];

  onMount(() => {
    const unsubscribe = componentRegistry.subscribe((next) => {
      components = next;
    });
    return () => unsubscribe();
  });
</script>

<!-- Generated components container - sits below the canvas but above the background -->
<div
  class="fixed inset-0 pointer-events-none z-10 overflow-hidden"
  style="right: {$panelWidth}px;"
>
  {#each components as config (config.id)}
    {#if config.component}
      <div
        class="pointer-events-auto absolute"
        style="left: {config.position.x}px; top: {config.position.y}px; width: {config.size.width}px; height: {config.size.height}px;"
      >
        <svelte:component this={config.component} {...(config.props ?? {})} />
      </div>
    {:else}
      <!-- Placeholder for components that failed to compile -->
      <div
        class="pointer-events-auto absolute flex items-center justify-center bg-neutral-800/50 border border-dashed border-neutral-600 rounded-lg text-neutral-500 text-sm"
        style="left: {config.position.x}px; top: {config.position.y}px; width: {config.size.width}px; height: {config.size.height}px;"
      >
        <span class="text-center p-2">
          Component: {config.id}
          <br />
          <span class="text-xs text-neutral-600">Loading...</span>
        </span>
      </div>
    {/if}
  {/each}
</div>
