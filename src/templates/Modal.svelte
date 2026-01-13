<!-- Modal dialog overlay component template -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open?: boolean;
    title?: string;
    size?: 'sm' | 'md' | 'lg' | 'xl';
    onclose?: () => void;
    children?: Snippet;
    footer?: Snippet;
  }

  let {
    open = false,
    title,
    size = 'md',
    onclose,
    children,
    footer,
  }: Props = $props();

  const sizeClasses = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
  };

  const handleBackdropClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      onclose?.();
    }
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      onclose?.();
    }
  };
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    onclick={handleBackdropClick}
    role="dialog"
    aria-modal="true"
  >
    <!-- Modal -->
    <div
      class="w-full {sizeClasses[size]} mx-4 bg-neutral-800 rounded-xl shadow-2xl border border-neutral-700"
    >
      <!-- Header -->
      {#if title}
        <div class="flex items-center justify-between px-6 py-4 border-b border-neutral-700">
          <h2 class="text-lg font-semibold text-neutral-100">{title}</h2>
          <button
            onclick={onclose}
            class="p-1 rounded-lg text-neutral-400 hover:text-neutral-100 hover:bg-neutral-700 transition-colors"
            aria-label="Close"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      {/if}

      <!-- Content -->
      <div class="px-6 py-4">
        {#if children}
          {@render children()}
        {/if}
      </div>

      <!-- Footer -->
      {#if footer}
        <div class="px-6 py-4 border-t border-neutral-700 flex justify-end gap-3">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}
