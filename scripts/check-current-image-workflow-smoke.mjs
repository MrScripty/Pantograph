#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(scriptPath), "..");
const retiredNodeTypes = new Set([
  "diffusion-inference",
  "llamacpp-inference",
  "pytorch-inference",
  "ollama-inference",
  "embedding",
  "reranker",
  "vision-analysis",
]);

function fail(message) {
  console.error(`[current-image-workflow-smoke] ${message}`);
  process.exit(1);
}

function requireObject(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail(`${label} must be an object`);
  }
  return value;
}

function requireArray(value, label) {
  if (!Array.isArray(value)) {
    fail(`${label} must be an array`);
  }
  return value;
}

function requireNode(nodes, id, nodeType) {
  const node = nodes.find((candidate) => candidate.id === id);
  if (!node) {
    fail(`missing node '${id}'`);
  }
  if (node.node_type !== nodeType) {
    fail(`node '${id}' must use '${nodeType}', found '${node.node_type}'`);
  }
  return node;
}

function requireEdge(edges, source, sourceHandle, target, targetHandle) {
  const edge = edges.find(
    (candidate) =>
      candidate.source === source &&
      candidate.source_handle === sourceHandle &&
      candidate.target === target &&
      candidate.target_handle === targetHandle,
  );
  if (!edge) {
    fail(
      `missing edge ${source}.${sourceHandle} -> ${target}.${targetHandle}`,
    );
  }
}

async function readWorkflow(relativePath) {
  const absolutePath = path.join(repoRoot, ...relativePath);
  return JSON.parse(await readFile(absolutePath, "utf8"));
}

function validateCanonicalImageGraph({
  graph,
  graphLabel,
  pumaNodeId,
  inferenceNodeId,
  imageOutputNodeId,
}) {
  const nodes = requireArray(graph.nodes, `${graphLabel}.nodes`);
  const edges = requireArray(graph.edges, `${graphLabel}.edges`);

  for (const node of nodes) {
    if (retiredNodeTypes.has(node.node_type)) {
      fail(`${graphLabel} contains retired node type '${node.node_type}'`);
    }
  }

  const pumaNode = requireNode(nodes, pumaNodeId, "puma-lib");
  const inferenceNode = requireNode(nodes, inferenceNodeId, "llm-inference");
  const imageOutputNode = requireNode(nodes, imageOutputNodeId, "image-output");

  if (inferenceNode.data?.task_kind !== "image_generation") {
    fail(`${graphLabel} llm-inference node must declare task_kind 'image_generation'`);
  }

  if (inferenceNode.data?.backend_key !== "pytorch") {
    fail(`${graphLabel} llm-inference node must declare backend_key 'pytorch'`);
  }

  if (typeof pumaNode.data?.label !== "string" || pumaNode.data.label.length === 0) {
    fail(`${graphLabel} puma-lib node must expose a visible label`);
  }

  if (
    typeof imageOutputNode.data?.label !== "string" ||
    imageOutputNode.data.label.length === 0
  ) {
    fail(`${graphLabel} image-output node must expose a visible label`);
  }

  requireEdge(
    edges,
    pumaNodeId,
    "pumas_model_ref",
    inferenceNodeId,
    "pumas_model_ref",
  );
  requireEdge(
    edges,
    pumaNodeId,
    "inference_settings",
    inferenceNodeId,
    "inference_settings",
  );
  requireEdge(edges, inferenceNodeId, "image", imageOutputNodeId, "image");
}

const starterWorkflow = await readWorkflow([
  "src",
  "templates",
  "workflows",
  "tiny-sd-turbo-text-to-image.json",
]);
const dataGraphs = requireObject(starterWorkflow.dataGraphs, "workflow.dataGraphs");
validateCanonicalImageGraph({
  graph: requireObject(
    dataGraphs["tiny-sd-turbo-generate"],
    "workflow.dataGraphs.tiny-sd-turbo-generate",
  ),
  graphLabel: "tiny-sd-turbo-generate",
  pumaNodeId: "model",
  inferenceNodeId: "diffusion",
  imageOutputNodeId: "image-output",
});

const juggernautWorkflow = await readWorkflow([
  ".pantograph",
  "workflows",
  "juggernaut-x-v10-sdxl.json",
]);
validateCanonicalImageGraph({
  graph: requireObject(juggernautWorkflow.graph, "juggernaut-x-v10-sdxl.graph"),
  graphLabel: "juggernaut-x-v10-sdxl",
  pumaNodeId: "puma-lib-juggernaut-x-v10",
  inferenceNodeId: "image-generation",
  imageOutputNodeId: "image-output",
});

console.log(
  "[current-image-workflow-smoke] current image workflow graph smoke passed",
);
