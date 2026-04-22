<script lang="ts">
  interface Props {
    mode: 'auto' | 'manual';
    onSetMode: (mode: 'auto' | 'manual') => void;
  }

  let { mode, onSetMode }: Props = $props();
</script>

<div class="flex gap-1 text-[10px]">
  <button
    type="button"
    class="px-2 py-0.5 rounded transition-colors {mode === 'auto'
      ? 'bg-cyan-600/30 text-cyan-300'
      : 'text-neutral-500 hover:text-neutral-300'}"
    onclick={() => onSetMode('auto')}
  >
    Auto
  </button>
  <button
    type="button"
    class="px-2 py-0.5 rounded transition-colors {mode === 'manual'
      ? 'bg-cyan-600/30 text-cyan-300'
      : 'text-neutral-500 hover:text-neutral-300'}"
    onclick={() => onSetMode('manual')}
  >
    Manual
  </button>
</div>
