<script lang="ts">
  import type { ComponentError } from '../types';

  interface Props {
    /** The error that occurred */
    error?: ComponentError | null;
    /** Error message string (alternative to error object) */
    errorMessage?: string;
    /** Component ID for display */
    componentId: string;
    /** Callback when retry button is clicked */
    onRetry?: () => void;
    /** Whether to show the retry button */
    showRetry?: boolean;
  }

  let {
    error = null,
    errorMessage,
    componentId,
    onRetry,
    showRetry = true,
  }: Props = $props();

  // Derive the display message
  const displayMessage = $derived(error?.errorMessage ?? errorMessage ?? 'Unknown error');

  // Derive error type label
  const errorTypeLabel = $derived(() => {
    if (!error?.errorType) return 'Error';
    switch (error.errorType) {
      case 'import':
        return 'Import Error';
      case 'validation':
        return 'Validation Error';
      case 'render':
        return 'Render Error';
      case 'timeout':
        return 'Timeout';
      default:
        return 'Error';
    }
  });

  function handleRetry() {
    onRetry?.();
  }
</script>

<div
  class="flex flex-col items-center justify-center h-full w-full bg-red-900/20 border border-dashed border-red-600/50 rounded-lg text-red-400 p-4 overflow-hidden"
>
  <!-- Error Type Badge -->
  <div class="flex items-center gap-2 mb-2">
    <span
      class="px-2 py-0.5 text-xs font-medium bg-red-800/50 text-red-300 rounded-full uppercase tracking-wide"
    >
      {errorTypeLabel()}
    </span>
  </div>

  <!-- Error Icon -->
  <div class="text-3xl mb-2 text-red-500">
    <svg
      xmlns="http://www.w3.org/2000/svg"
      class="h-8 w-8"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      stroke-width="2"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
      />
    </svg>
  </div>

  <!-- Error Message -->
  <p class="text-sm text-center text-red-300 mb-2 max-w-full break-words line-clamp-3">
    {displayMessage}
  </p>

  <!-- Component ID -->
  <p class="text-xs text-red-600/70 font-mono mb-3 truncate max-w-full">
    {componentId}
  </p>

  <!-- Retry Button -->
  {#if showRetry && onRetry}
    <button
      onclick={handleRetry}
      class="px-3 py-1 text-xs font-medium bg-red-800/50 hover:bg-red-700/50 text-red-200 rounded transition-colors"
    >
      Retry
    </button>
  {/if}

  <!-- Timestamp -->
  {#if error?.timestamp}
    <p class="text-xs text-red-600/50 mt-2">
      {new Date(error.timestamp).toLocaleTimeString()}
    </p>
  {/if}
</div>

<style>
  .line-clamp-3 {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
