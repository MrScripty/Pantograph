<script lang="ts">
  import type { WorkflowGraphCurrentValidationSummaryResponse } from '../services/workflow/types';
  import { currentWorkflowValidation } from './workflowValidationAuthority';

  interface Props {
    validation: WorkflowGraphCurrentValidationSummaryResponse | null;
    graphSessionId: string | null;
    graphRevision: string | null;
  }
  let { validation, graphSessionId, graphRevision }: Props = $props();
  // A diagnostic from a previous graph/session must never describe the active editor.
  let current = $derived(currentWorkflowValidation(validation, graphSessionId, graphRevision));
  let failures = $derived(current?.group_preflight?.failures ?? []);
  let diagnostics = $derived((current?.diagnostics ?? []).filter(diagnostic =>
    !failures.some(failure => failure.code === diagnostic.code && failure.message === diagnostic.message)));
</script>

{#if failures.length || diagnostics.length}
  <section aria-label="Workflow validation diagnostics" class="border-b border-neutral-700 bg-neutral-900 px-4 py-3 text-xs">
    <h2 class="mb-2 font-medium text-neutral-200">Workflow validation: {current.state}</h2>
    <ul class="space-y-3">
      {#each failures as failure, index (index)}
        <li class="break-words text-neutral-200" data-testid="workflow-group-diagnostic">
          <p><span class="font-semibold">{failure.rejection_kind}</span> <code>{failure.code}</code></p>
          <p>{#if failure.group_id}Group: {failure.group_id} {/if}{#if failure.child_id}Child: {failure.child_id} {/if}Field: {failure.field}</p>
          <p class="mt-1">{failure.message}</p>
          <p class="mt-1 text-cyan-200">How to fix: {failure.repair_hint}</p>
        </li>
      {/each}
      {#each diagnostics as diagnostic, index (index)}
        <li class="break-words text-neutral-200" data-testid="workflow-validation-diagnostic">
          <p>
            <span class="font-semibold">{diagnostic.severity}</span>
            <code class="ml-2 text-neutral-400">{diagnostic.code}</code>
            {#if diagnostic.port_id}<span class="ml-2">Port: {diagnostic.port_id}</span>{/if}
          </p>
          <p class="mt-1">{diagnostic.message}</p>
          {#if diagnostic.hint}<p class="mt-1 text-cyan-200">How to fix: {diagnostic.hint}</p>{/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}
