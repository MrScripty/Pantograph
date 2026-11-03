<script lang="ts">
  import StatusGlyph from './StatusGlyph.svelte';
  import { glyphStateForLogLevel, type LogLevel } from './statusGlyph';

  interface Props {
    /** Mono timestamp, for example "19:35:42.118". */
    time: string;
    level: LogLevel;
    /** Bold event kind, for example "runtime.model_load_failed". */
    kind?: string;
    message: string;
    /** Muted suffix naming where the event came from. */
    source?: string;
    /** Focused line (Diagnostics deep link) gets the selected-row treatment. */
    selected?: boolean;
    id?: string;
    /** Hover text, for example the event id. */
    title?: string;
  }

  let { time, level, kind, message, source, selected = false, id, title }: Props = $props();

  const LEVEL_TEXT: Record<LogLevel, string> = {
    info: 'Info',
    warn: 'Warning',
    error: 'Error',
  };
</script>

<div class="pg-ln" class:pg-ln-err={level === 'error'} {id} {title} aria-current={selected ? 'true' : undefined}>
  <span class="pg-t">{time}</span>
  <StatusGlyph state={glyphStateForLogLevel(level)} />
  <span class="pg-m"><span class="pg-sr-only">{`${LEVEL_TEXT[level]}: `}</span>{#if kind}<b>{kind}</b>{` ${message}`}{:else}{message}{/if}{#if source}<span class="pg-muted">{` ${source}`}</span>{/if}</span>
</div>
