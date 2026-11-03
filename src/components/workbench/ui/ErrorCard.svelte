<script lang="ts">
  import StatusGlyph from './StatusGlyph.svelte';

  interface Props {
    /** Short sentence-case title, for example "Out of memory in load_model". */
    title: string;
    /** Muted machine detail, for example the error code and phase. */
    code?: string;
    /** One sentence saying what happened. */
    explanation: string;
    /** One sentence of advice; "Try: " is added when missing. */
    suggestion?: string;
    actionLabel?: string;
    onaction?: () => void;
    /** "danger" (default) for a failure; "muted" for a stale or advisory fact. */
    tone?: 'danger' | 'muted';
    testId?: string;
  }

  let { title, code, explanation, suggestion, actionLabel, onaction, tone = 'danger', testId }: Props = $props();

  let suggestionText = $derived(
    suggestion ? (suggestion.startsWith('Try:') ? suggestion : `Try: ${suggestion}`) : null,
  );
</script>

<div class="pg-errcard" class:pg-errcard-muted={tone === 'muted'} data-testid={testId}>
  <StatusGlyph state={tone === 'muted' ? 'idle' : 'failed'} />
  <div class="min-w-0 flex-1">
    <div><b class="pg-errcard-title">{title}</b>{#if code}<span class="pg-muted pg-small pg-errcard-code">{code}</span>{/if}</div>
    <div class="pg-small pg-errcard-text">{explanation}</div>
    {#if suggestionText}
      <div class="pg-small pg-muted pg-errcard-hint">{suggestionText}</div>
    {/if}
  </div>
  {#if actionLabel && onaction}
    <button class="pg-btn pg-btn-sm" type="button" onclick={onaction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .pg-errcard-title {
    color: var(--pg-text-bright);
  }

  .pg-errcard-code {
    margin-left: 6px;
  }

  .pg-errcard-text {
    margin-top: 2px;
  }

  .pg-errcard-hint {
    margin-top: 6px;
  }
</style>
