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
  const template = loadTemplate('../../templates/workflows/gguf-reranker-workflow.json');

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
});
