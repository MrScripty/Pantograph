<script lang="ts">
  import { SlidersHorizontal } from 'lucide-svelte';
  import { closeSidePanel } from '../../stores/panelStore';
  import { setWorkbenchPage } from '../../stores/workbenchStore';

  function openWorkbenchSettings(): void {
    setWorkbenchPage('settings');
    closeSidePanel();
  }
</script>

<div class="flex flex-1 flex-col justify-center px-4 py-6">
  <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
    <div class="flex items-center gap-2 text-sm font-medium text-neutral-100">
      <SlidersHorizontal size={16} aria-hidden="true" class="text-cyan-300" />
      Settings
    </div>
    <p class="mt-2 text-xs leading-5 text-neutral-400">
      Persistent configuration is managed in the workbench Settings page.
    </p>
    <button
      type="button"
      class="mt-4 inline-flex w-full items-center justify-center gap-2 rounded border border-cyan-800 bg-cyan-950 px-3 py-2 text-sm text-cyan-100 transition-colors hover:border-cyan-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400"
      onclick={openWorkbenchSettings}
    >
      Open Workbench Settings
    </button>
  </div>
</div>
