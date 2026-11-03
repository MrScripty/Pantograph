<script lang="ts">
  interface Props {
    /** Text to show; null hides the toast but keeps the live region mounted. */
    message: string | null;
  }

  let { message }: Props = $props();
</script>

<!-- The region stays mounted so screen readers announce each new message. -->
<div class={message ? 'pg-toast' : 'pg-sr-only'} role="status" aria-live="polite">{message ?? ''}</div>
