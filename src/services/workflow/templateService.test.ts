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

interface SavedWorkflow {
  metadata: {
    name: string;
  };
  graph: TemplateGraph;
}

function loadTemplate(relativePath: string): WorkflowTemplate {
  const currentDir = dirname(fileURLToPath(import.meta.url));
  const templatePath = resolve(currentDir, relativePath);
  return JSON.parse(readFileSync(templatePath, 'utf8')) as WorkflowTemplate;
}

function loadSavedWorkflow(relativePath: string): SavedWorkflow {
  const currentDir = dirname(fileURLToPath(import.meta.url));
  const workflowPath = resolve(currentDir, relativePath);
  return JSON.parse(readFileSync(workflowPath, 'utf8')) as SavedWorkflow;
}

const builtInTemplatePaths = [
  '../../templates/workflows/gguf-reranker-workflow.json',
  '../../templates/workflows/svelte-code-agent.json',
  '../../templates/workflows/tiny-sd-turbo-text-to-image.json',
];

const trackedSavedWorkflowPaths = [
  '../../../.pantograph/workflows/coding-agent.json',
  '../../../.pantograph/workflows/juggernaut-x-v10-sdxl.json',
  '../../../.pantograph/workflows/tiny-sd-turbo-diffusion.json',
];

test('built-in templates do not use retired direct inference nodes', () => {
  const retiredNodeTypes = new Set([
    'diffusion-inference',
    'embedding',
    'llamacpp-inference',
    'ollama-inference',
    'pytorch-inference',
    'reranker',
  ]);

  for (const templatePath of builtInTemplatePaths) {
    const template = loadTemplate(templatePath);
    for (const graph of Object.values(template.dataGraphs)) {
      for (const node of graph.nodes) {
        assert.equal(
          retiredNodeTypes.has(node.node_type),
          false,
          `${templatePath} must not use retired inference node type ${node.node_type}`,
        );
      }
    }
  }
});

test('puma-lib to canonical inference template edges carry package facts', () => {
  const templates = builtInTemplatePaths.map(loadTemplate);

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
        node.data?.backend_key === 'pytorch',
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

test('tracked image-generation workflows use canonical llm inference shape', () => {
  const workflows = trackedSavedWorkflowPaths.map(loadSavedWorkflow);
  const imageWorkflows = workflows.filter((workflow) =>
    /juggernaut|tiny sd turbo/i.test(workflow.metadata.name),
  );

  assert.equal(
    imageWorkflows.filter((workflow) => /juggernaut/i.test(workflow.metadata.name)).length,
    1,
    'tracked saved workflows must contain exactly one Juggernaut example',
  );

  for (const workflow of imageWorkflows) {
    const nodes = workflow.graph.nodes;
    const edges = workflow.graph.edges;
    const nodeTypesById = new Map(nodes.map((node) => [node.id, node.node_type]));
    const pumaNode = nodes.find((node) => node.node_type === 'puma-lib');
    const inferenceNode = nodes.find((node) => node.node_type === 'llm-inference');

    assert.ok(pumaNode, `${workflow.metadata.name} must include a Puma-Lib model node`);
    assert.ok(inferenceNode, `${workflow.metadata.name} must include canonical llm-inference`);
    assert.equal(
      nodes.some((node) => node.node_type === 'diffusion-inference'),
      false,
      `${workflow.metadata.name} must not use retired direct diffusion inference`,
    );
    assert.equal(inferenceNode.data?.task_kind, 'image_generation');
    assert.equal(inferenceNode.data?.backend_key, 'pytorch');
    assert.equal(inferenceNode.data?.runtime_hint, undefined);
    assert.equal(
      typeof pumaNode.data?.model_id,
      'string',
      `${workflow.metadata.name} must persist stable Pumas model identity`,
    );
    assert.equal(
      Object.hasOwn(pumaNode.data ?? {}, 'modelPath'),
      false,
      `${workflow.metadata.name} must not persist raw local Pumas paths`,
    );
    assert.equal(
      Object.hasOwn(pumaNode.data ?? {}, 'dependency_requirements'),
      false,
      `${workflow.metadata.name} must not persist derived dependency snapshots`,
    );
    assert.ok(
      edges.some(
        (edge) =>
          nodeTypesById.get(edge.source) === 'puma-lib' &&
          nodeTypesById.get(edge.target) === 'llm-inference' &&
          edge.source_handle === 'pumas_model_ref' &&
          edge.target_handle === 'pumas_model_ref',
      ),
      `${workflow.metadata.name} must route pumas_model_ref into image generation`,
    );
    assert.ok(
      edges.some(
        (edge) =>
          nodeTypesById.get(edge.source) === 'puma-lib' &&
          nodeTypesById.get(edge.target) === 'llm-inference' &&
          edge.source_handle === 'resolved_model_package_facts' &&
          edge.target_handle === 'resolved_model_package_facts',
      ),
      `${workflow.metadata.name} must route package facts into image generation`,
    );
    assert.ok(
      edges.some(
        (edge) =>
          nodeTypesById.get(edge.source) === 'llm-inference' &&
          nodeTypesById.get(edge.target) === 'image-output' &&
          edge.source_handle === 'image' &&
          edge.target_handle === 'image',
      ),
      `${workflow.metadata.name} must route generated image output to image-output`,
    );
  }
});
