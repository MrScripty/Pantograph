<script lang="ts">
  let {
    trace,
    displayState,
    blockedReason = null,
  }: {
    trace: string;
    displayState: string;
    blockedReason?: string | null;
  } = $props();
</script>

<div class="horseshoe-debug">
  <div>trace: {trace}</div>
  <div>state: {displayState}</div>
  {#if blockedReason}
    <div>blocked: {blockedReason}</div>
  {/if}
</div>

<style>
  .horseshoe-debug {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    z-index: 1400;
    pointer-events: none;
    padding: 0.5rem 0.65rem;
    border-radius: 0.5rem;
    background: rgba(10, 10, 10, 0.88);
    border: 1px solid rgba(82, 82, 91, 0.9);
    color: #e5e7eb;
    font-size: 0.72rem;
    line-height: 1.35;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
  }
</style>
