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
      ? 'bg-pg-panel-selected text-pg-text'
      : 'text-pg-text-muted hover:text-pg-text'}"
    aria-pressed={mode === 'auto'}
    onclick={() => onSetMode('auto')}
  >
    Auto
  </button>
  <button
    type="button"
    class="px-2 py-0.5 rounded transition-colors {mode === 'manual'
      ? 'bg-pg-panel-selected text-pg-text'
      : 'text-pg-text-muted hover:text-pg-text'}"
    aria-pressed={mode === 'manual'}
    onclick={() => onSetMode('manual')}
  >
    Manual
  </button>
</div>
