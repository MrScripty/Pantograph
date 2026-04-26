import type { WorkflowDerivedGraph, WorkflowGraph } from './types/workflow.js';

const DERIVED_GRAPH_SCHEMA_VERSION = 1;
const FNV64_OFFSET_BASIS = BigInt('0xcbf29ce484222325');
const FNV64_PRIME = BigInt('0x100000001b3');
const FNV64_MASK = BigInt('0xffffffffffffffff');
const ENCODER = new TextEncoder();

function fnv1a64Update(hash: bigint, input: string): bigint {
  const bytes = ENCODER.encode(input);
  let nextHash = hash;
  for (const byte of bytes) {
    nextHash ^= BigInt(byte);
    nextHash = (nextHash * FNV64_PRIME) & FNV64_MASK;
  }
  return nextHash;
}

export function computeGraphFingerprint(graph: WorkflowGraph): string {
  const nodeRows = graph.nodes
    .map((node) => `${node.id}|${node.node_type}`)
    .sort();

  const edgeRows = graph.edges
    .map(
      (edge) =>
        `${edge.source}|${edge.source_handle}|${edge.target}|${edge.target_handle}`
    )
    .sort();

  let digest = FNV64_OFFSET_BASIS;
  digest = fnv1a64Update(digest, 'v1');
  for (const row of nodeRows) {
    digest = fnv1a64Update(digest, row);
    digest = fnv1a64Update(digest, '\n');
  }
  digest = fnv1a64Update(digest, '--');
  for (const row of edgeRows) {
    digest = fnv1a64Update(digest, row);
    digest = fnv1a64Update(digest, '\n');
  }
  return digest.toString(16).padStart(16, '0');
}

export function computeConsumerCountMap(graph: WorkflowGraph): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const edge of graph.edges) {
    const key = `${edge.source}:${edge.source_handle}`;
    counts[key] = (counts[key] ?? 0) + 1;
  }
  return counts;
}

export function buildDerivedGraph(graph: WorkflowGraph): WorkflowDerivedGraph {
  return {
    schema_version: DERIVED_GRAPH_SCHEMA_VERSION,
    graph_fingerprint: computeGraphFingerprint(graph),
    consumer_count_map: computeConsumerCountMap(graph),
  };
}

export function withDerivedGraph(graph: WorkflowGraph): WorkflowGraph {
  return {
    ...graph,
    derived_graph: buildDerivedGraph(graph),
  };
}
