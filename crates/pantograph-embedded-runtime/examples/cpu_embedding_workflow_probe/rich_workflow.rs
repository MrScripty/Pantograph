//! Bounded multi-branch fixture graph using actual public runtime contracts.
//! Image generation and external image input remain explicitly unavailable.
use super::*;

pub(super) const WORKFLOW: &str = "cpu-multibranch-probe";
pub(super) const SECOND_MODEL: &str = "embedding/qualification/synthetic-bert-12";
pub(super) const IMAGE_STATUS: &str = "Image unavailable: no diffusion weights or typed external image source. Text generation is unqualified: Python dependency handoff is not implemented in this host.";

pub(super) fn graph() -> ProbeResult<WorkflowGraph> {
    let nodes = json!([
        {"id":"prompt","node_type":"text-input","position":{"x":0,"y":80},"data":{"label":"Prompt","text":"hello world"}},
        {"id":"context","node_type":"text-input","position":{"x":0,"y":350},"data":{"label":"Context","text":"different world"}},
        {"id":"compose","node_type":"merge","position":{"x":310,"y":180},"data":{"label":"Compose prompt","separator":"\n"}},
        {"id":"context-infer","node_type":"llm-inference","position":{"x":680,"y":-110},"data":{
            "label":"Context embedding (fixture)","modelName":"Untrained BERT-12 · CPU",
            "task_kind":"embedding","runtime":"candle","device":"cpu","emit_metadata":false,
            "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":SECOND_MODEL,"selected_artifact_id":"main"}}},
        {"id":"context-deps","node_type":"dependency-environment","position":{"x":680,"y":230},"data":{"mode":"manual","label":"Context embedding dependencies"}},
        {"id":"context-vector","node_type":"vector-output","position":{"x":1110,"y":-80},"data":{"label":"Context vector (12 dimensions)"}},
        {"id":"infer","node_type":"llm-inference","position":{"x":680,"y":790},"data":{
            "label":"Embedding (fixture)","modelName":"Untrained BERT-8 · CPU",
            "task_kind":"embedding","runtime":"candle","device":"cpu","emit_metadata":true,
            "runtime_source_context":{"operation_type":"embedding.text","context_shape_key":"embedding.one-text","cancellation_mode":"run_scoped"},
            "pumas_model_ref":{"model_id":MODEL_ID,"selected_artifact_id":"main"}}},
        {"id":"deps","node_type":"dependency-environment","position":{"x":680,"y":1080},"data":{"mode":"manual","label":"Embedding dependencies"}},
        {"id":"vectors","node_type":"vector-output","position":{"x":1110,"y":790},"data":{"label":"Embedding vector"}},
        {"id":"model-info","node_type":"json-filter","position":{"x":1110,"y":1010},"data":{"label":"Actual runtime identity","path":"runtime_variant_id"}},
        {"id":"image-status","node_type":"text-input","position":{"x":1110,"y":1240},"data":{"label":"Unavailable capabilities","text":IMAGE_STATUS}},
        {"id":"report-merge","node_type":"merge","position":{"x":1560,"y":500},"data":{"label":"Combine branch report","separator":"\n"}},
        {"id":"report","node_type":"text-output","position":{"x":1880,"y":500},"data":{"label":"Combined execution report"}}
    ]);
    let links = [
        ("prompt", "text", "compose", "inputs"),
        ("context", "text", "compose", "inputs"),
        ("context", "text", "context-infer", "text"),
        (
            "context-deps",
            "dependency_environment_sidecar",
            "context-infer",
            "dependency_environment_sidecar",
        ),
        ("context-infer", "embedding", "context-vector", "vector"),
        ("compose", "merged", "infer", "text"),
        (
            "deps",
            "dependency_environment_sidecar",
            "infer",
            "dependency_environment_sidecar",
        ),
        ("infer", "embedding", "vectors", "vector"),
        ("infer", "metadata", "model-info", "json"),
        ("model-info", "value", "report-merge", "inputs"),
        ("compose", "merged", "report-merge", "inputs"),
        ("image-status", "text", "report-merge", "inputs"),
        ("report-merge", "merged", "report", "text"),
    ];
    let edges: Vec<_> = links.into_iter().enumerate().map(|(i,(source,source_handle,target,target_handle))|
        json!({"id":format!("rich-edge-{i}"),"source":source,"source_handle":source_handle,"target":target,"target_handle":target_handle})
    ).collect();
    Ok(serde_json::from_value(
        json!({"nodes":nodes,"edges":edges}),
    )?)
}

pub(super) fn validate(graph: &WorkflowGraph) -> ProbeResult<()> {
    let expected = self::graph()?;
    if graph.nodes.len() != expected.nodes.len() || graph.edges != expected.edges {
        return Err("The rich fixture topology is fixed".into());
    }
    for original in expected.nodes {
        let node = graph
            .nodes
            .iter()
            .find(|n| n.id == original.id && n.node_type == original.node_type)
            .ok_or("The rich fixture nodes are fixed")?;
        let keys: &[&str] = match node.node_type.as_str() {
            "llm-inference" => &[
                "task_kind",
                "runtime",
                "device",
                "pumas_model_ref",
                "emit_metadata",
                "runtime_source_context",
            ],
            "json-filter" => &["path"],
            "text-input" if node.id == "image-status" => &["text"],
            _ => &[],
        };
        if keys
            .iter()
            .any(|key| node.data[*key] != original.data[*key])
        {
            return Err(
                "The rich fixture model bindings, controls and reference provenance are fixed"
                    .into(),
            );
        }
    }
    Ok(())
}
