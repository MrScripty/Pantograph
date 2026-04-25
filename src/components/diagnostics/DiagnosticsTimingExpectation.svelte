<script lang="ts">
  import type { WorkflowTimingExpectation } from '../../services/diagnostics/types';
  import {
    formatTimingExpectationDetail,
    formatTimingExpectationSummary,
    getTimingExpectationClasses,
  } from './presenters';

  type Props = {
    expectation?: WorkflowTimingExpectation | null;
    align?: 'left' | 'right';
  };

  let { expectation = null, align = 'left' }: Props = $props();
</script>

<div class={`flex flex-col gap-1 ${align === 'right' ? 'items-end text-right' : 'items-start text-left'}`}>
  <span class={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${getTimingExpectationClasses(expectation?.comparison)}`}>
    {formatTimingExpectationSummary(expectation)}
  </span>
  <span class="max-w-64 text-[11px] leading-snug text-neutral-500">
    {formatTimingExpectationDetail(expectation)}
  </span>
</div>
