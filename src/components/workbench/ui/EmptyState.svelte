<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Short serif question or statement, for example "Which run do you want to look into?". */
    question: string;
    /** One sentence of context under the question. */
    sentence?: string;
    /** Primary action label, for example "Pick a run". */
    actionLabel?: string;
    onaction?: () => void;
    /** Accessible name of the region; defaults to the question. */
    label?: string;
    /** Extra content under the action, for example a secondary disclosure. */
    children?: Snippet;
  }

  let { question, sentence, actionLabel, onaction, label, children }: Props = $props();
</script>

<section class="pg-panel pg-emptystate" aria-label={label ?? question}>
  <div class="pg-emptystate-body">
    <div class="pg-subtitle">{question}</div>
    {#if sentence}
      <div class="pg-muted pg-small pg-emptystate-sentence">{sentence}</div>
    {/if}
    {#if actionLabel && onaction}
      <button class="pg-btn pg-btn-primary" type="button" onclick={onaction}>{actionLabel}</button>
    {/if}
    {#if children}
      <div class="pg-emptystate-extra">{@render children()}</div>
    {/if}
  </div>
</section>

<style>
  .pg-emptystate-extra {
    margin-top: 18px;
    text-align: left;
  }

  .pg-emptystate {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .pg-emptystate-body {
    text-align: center;
    max-width: 420px;
  }

  .pg-emptystate-sentence {
    margin: 6px 0 14px;
  }
</style>
