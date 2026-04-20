<script lang="ts">
  type Props = {
    connectionType: 'external' | 'sidecar';
  };

  let { connectionType = $bindable() }: Props = $props();
</script>

<div class="flex gap-1 rounded bg-neutral-900 p-1">
  <button
    type="button"
    class={`flex-1 rounded px-2 py-1 text-xs transition-colors ${
      connectionType === 'external'
        ? 'bg-neutral-700 text-neutral-200'
        : 'text-neutral-500 hover:text-neutral-400'
    }`}
    onclick={() => (connectionType = 'external')}
  >
    External
  </button>
  <button
    type="button"
    class={`flex-1 rounded px-2 py-1 text-xs transition-colors ${
      connectionType === 'sidecar'
        ? 'bg-neutral-700 text-neutral-200'
        : 'text-neutral-500 hover:text-neutral-400'
    }`}
    onclick={() => (connectionType = 'sidecar')}
  >
    Sidecar
  </button>
</div>
