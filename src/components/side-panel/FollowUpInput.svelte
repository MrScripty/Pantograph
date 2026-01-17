<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isAgentRunning: boolean = false;
  export let isReady: boolean = false;

  const dispatch = createEventDispatcher<{
    submit: string;
    stop: void;
  }>();

  let followUpInput = '';

  const handleFollowUp = () => {
    if (!followUpInput.trim() || isAgentRunning) return;
    dispatch('submit', followUpInput.trim());
    followUpInput = '';
  };

  const handleStopAgent = () => {
    dispatch('stop');
  };

  const handleFollowUpKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleFollowUp();
    }
  };
</script>

<div class="px-4 py-3 border-t border-neutral-700">
  <div class="flex bg-neutral-800 border border-neutral-600 rounded-lg overflow-hidden">
    <input
      type="text"
      bind:value={followUpInput}
      placeholder={isAgentRunning ? "Agent is running..." : "Send a message..."}
      class="flex-1 bg-transparent px-3 py-2 outline-none font-mono text-sm placeholder:text-neutral-600"
      disabled={isAgentRunning}
      on:keydown={handleFollowUpKeyDown}
    />
    {#if isAgentRunning}
      <!-- Stop button when agent is running -->
      <button
        on:click={handleStopAgent}
        class="px-4 py-2 bg-red-700 hover:bg-red-600 border-l border-neutral-600 transition-colors text-xs font-bold tracking-wider text-white"
        title="Stop agent"
      >
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
          <rect x="6" y="6" width="12" height="12" rx="1" />
        </svg>
      </button>
    {:else}
      <!-- Send button when agent is idle -->
      <button
        on:click={handleFollowUp}
        disabled={!followUpInput.trim() || !isReady}
        class="px-4 py-2 bg-neutral-700 hover:bg-neutral-600 disabled:opacity-50 disabled:cursor-not-allowed border-l border-neutral-600 transition-colors text-xs font-bold tracking-wider"
        title="Send message"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
        </svg>
      </button>
    {/if}
  </div>
</div>
