import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

interface TemplateEdge {
  source: string;
  source_handle: string;
  target: string;
  target_handle: string;
}

interface TemplateNode {
  id: string;
  node_type: string;
  data?: Record<string, unknown>;
}

interface TemplateGraph {
  nodes: TemplateNode[];
  edges: TemplateEdge[];
}

interface WorkflowTemplate {
  dataGraphs: Record<string, TemplateGraph>;
}

function loadTemplate(relativePath: string): WorkflowTemplate {
  const currentDir = dirname(fileURLToPath(import.meta.url));
  const templatePath = resolve(currentDir, relativePath);
  return JSON.parse(readFileSync(templatePath, 'utf8')) as WorkflowTemplate;
}

test('puma-lib to canonical inference template edges carry package facts', () => {
  const templates = [
    loadTemplate('../../templates/workflows/gguf-reranker-workflow.json'),
    loadTemplate('../../templates/workflows/tiny-sd-turbo-text-to-image.json'),
  ];

  for (const template of templates) {
    for (const graph of Object.values(template.dataGraphs)) {
      const nodeTypesById = new Map(graph.nodes.map((node) => [node.id, node.node_type]));
      const pumaToInferenceEdges = graph.edges.filter(
        (edge) =>
          nodeTypesById.get(edge.source) === 'puma-lib' &&
          nodeTypesById.get(edge.target) === 'llm-inference',
      );

      if (pumaToInferenceEdges.length === 0) {
        continue;
      }

      assert.ok(
        pumaToInferenceEdges.some(
          (edge) =>
            edge.source_handle === 'pumas_model_ref' && edge.target_handle === 'pumas_model_ref',
        ),
        'puma-lib to llm-inference template edges must carry pumas_model_ref',
      );
      assert.ok(
        pumaToInferenceEdges.some(
          (edge) =>
            edge.source_handle === 'resolved_model_package_facts' &&
            edge.target_handle === 'resolved_model_package_facts',
        ),
        'puma-lib to llm-inference template edges must carry resolved_model_package_facts',
      );
    }
  }
});

test('tiny sd template uses canonical image-generation inference', () => {
  const template = loadTemplate('../../templates/workflows/tiny-sd-turbo-text-to-image.json');
  const graphs = Object.values(template.dataGraphs);
  const nodes = graphs.flatMap((graph) => graph.nodes);
  const edges = graphs.flatMap((graph) => graph.edges);

  assert.equal(
    nodes.some((node) => node.node_type === 'diffusion-inference'),
    false,
    'tiny sd template must not use retired direct diffusion inference',
  );
  assert.ok(
    nodes.some(
      (node) =>
        node.node_type === 'llm-inference' &&
        node.data?.task_kind === 'image_generation' &&
        node.data?.runtime_hint === 'diffusers',
    ),
    'tiny sd template must use canonical llm-inference',
  );
  assert.equal(
    edges.some(
      (edge) =>
        edge.source === 'model' &&
        edge.target === 'diffusion' &&
        (edge.source_handle === 'model_path' || edge.target_handle === 'model_path'),
    ),
    false,
    'tiny sd template must not hand raw model_path to canonical inference',
  );
  assert.ok(
    edges.some(
      (edge) =>
        edge.source === 'diffusion' &&
        edge.source_handle === 'image' &&
        edge.target === 'image-output' &&
        edge.target_handle === 'image',
    ),
    'tiny sd template must route canonical image output to image-output',
  );
});
