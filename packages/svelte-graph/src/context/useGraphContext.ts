/**
 * Retrieve the graph context set by a parent component via createGraphContext().
 *
 * Must be called during component initialization (top-level <script>).
 * Throws if no context is found (meaning createGraphContext wasn't called in a parent).
 */
import { getContext } from 'svelte';
import { GRAPH_CONTEXT_KEY } from './keys.js';
import type { GraphContext } from './types.js';

export function useGraphContext(): GraphContext {
  const context = getContext<GraphContext>(GRAPH_CONTEXT_KEY);
  if (!context) {
    throw new Error(
      'useGraphContext() called outside of a GraphContext provider. ' +
      'Call createGraphContext() in a parent component first.'
    );
  }
  return context;
}
