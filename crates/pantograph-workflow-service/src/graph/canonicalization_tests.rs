use std::collections::HashSet;

use pantograph_node_contracts::{
    ContractUpgradeChange, ContractUpgradeOutcome, DiagnosticsLineagePolicy, PortKind,
};
use serde_json::json;

use super::super::registry::NodeRegistry;
use super::super::types::{GraphEdge, GraphNode, WorkflowGraph};
use super::legacy_migration::{legacy_inference_node_migration_specs, LegacyInferencePortAction};
use super::{canonicalize_workflow_graph, canonicalize_workflow_graph_with_migrations};

#[test]
fn canonicalize_workflow_graph_migrates_legacy_system_prompt_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "system-prompt".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "prompt": "hello" }),
            },
            GraphNode {
                id: "target".to_string(),
                node_type: "llm-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({}),
            },
        ],
        edges: vec![GraphEdge {
            id: "prompt-prompt-target-prompt".to_string(),
            source: "prompt".to_string(),
            source_handle: "prompt".to_string(),
            target: "target".to_string(),
            target_handle: "prompt".to_string(),
        }],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let prompt_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "prompt")
        .expect("prompt node");
    assert_eq!(prompt_node.node_type, "text-input");
    assert_eq!(prompt_node.data["text"], json!("hello"));
    assert_eq!(canonical.edges[0].source_handle, "text");
    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "system-prompt");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert_eq!(
        record.diagnostics_lineage,
        DiagnosticsLineagePolicy::PreservePrimitiveLineage
    );
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::NodeTypeChanged { from, to, .. }
            if from.as_str() == "system-prompt" && to.as_str() == "text-input"
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortIdChanged { from, to, kind, .. }
            if from.as_str() == "prompt"
                && to.as_str() == "text"
                && *kind == PortKind::Output
    )));
}

#[test]
fn canonicalize_workflow_graph_migrates_legacy_ollama_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "model".to_string(),
                node_type: "model-provider".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "model_name": "llama3:8b" }),
            },
            GraphNode {
                id: "ollama".to_string(),
                node_type: "ollama-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model": "llama3:8b",
                    "temperature": 0.2,
                    "max_tokens": 128,
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "model-ref-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 100.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "model-name-ollama-model".to_string(),
                source: "model".to_string(),
                source_handle: "model_name".to_string(),
                target: "ollama".to_string(),
                target_handle: "model".to_string(),
            },
            GraphEdge {
                id: "ollama-response-output-text".to_string(),
                source: "ollama".to_string(),
                source_handle: "response".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "ollama-model-ref-output-text".to_string(),
                source: "ollama".to_string(),
                source_handle: "model_ref".to_string(),
                target: "model-ref-output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let migrated = canonical
        .nodes
        .iter()
        .find(|node| node.id == "ollama")
        .expect("migrated ollama node");

    assert_eq!(migrated.node_type, "llm-inference");
    assert_eq!(migrated.data["task_kind"], json!("text_generation"));
    assert_eq!(migrated.data["runtime_hint"], json!("retired_ollama"));
    assert_eq!(
        migrated.data["pumas_model_ref"]["status"],
        json!("unresolved")
    );
    assert_eq!(
        migrated.data["pumas_model_ref"]["legacy_model"],
        json!("llama3:8b")
    );
    assert_eq!(
        migrated.data["migration_diagnostics"][0]["code"],
        json!("legacy_ollama_backend_retired")
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "ollama-response-output-text"
            && edge.source_handle == "response"
            && edge.target_handle == "text"
    }));
    assert!(!canonical
        .edges
        .iter()
        .any(|edge| edge.target == "ollama" && edge.target_handle == "model"));
    assert!(!canonical
        .edges
        .iter()
        .any(|edge| edge.source == "ollama" && edge.source_handle == "model_ref"));

    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "ollama-inference");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert_eq!(
        record.diagnostics_lineage,
        DiagnosticsLineagePolicy::RejectToAvoidSilentChange
    );
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::NodeTypeChanged { from, to, .. }
            if from.as_str() == "ollama-inference" && to.as_str() == "llm-inference"
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model" && *kind == PortKind::Input
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model_ref" && *kind == PortKind::Output
    )));
    assert!(record
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Ollama execution is retired")));

    let ollama_spec = legacy_inference_node_migration_specs()
        .iter()
        .find(|spec| spec.legacy_node_type == "ollama-inference")
        .expect("ollama migration spec");
    assert_eq!(
        ollama_spec.task_kind.as_str(),
        migrated.data["task_kind"].as_str().expect("task kind")
    );
    assert_eq!(
        ollama_spec.runtime_hint.as_str(),
        migrated.data["runtime_hint"]
            .as_str()
            .expect("runtime hint")
    );
    assert_eq!(
        ollama_spec.diagnostic_code,
        migrated.data["migration_diagnostics"][0]["code"]
            .as_str()
            .expect("diagnostic code")
    );
}

#[test]
fn legacy_inference_migration_inventory_covers_planned_node_types() {
    let specs = legacy_inference_node_migration_specs();
    let node_types = specs
        .iter()
        .map(|spec| spec.legacy_node_type)
        .collect::<HashSet<_>>();

    for expected in [
        "ollama-inference",
        "llamacpp-inference",
        "pytorch-inference",
        "embedding",
        "reranker",
        "llm-inference",
    ] {
        assert!(
            node_types.contains(expected),
            "missing migration inventory for {expected}"
        );
    }
}

#[test]
fn legacy_inference_migration_inventory_defines_canonical_data_fields() {
    for spec in legacy_inference_node_migration_specs() {
        assert_eq!(spec.canonical_node_type, "llm-inference");
        assert!(spec.node_data_fields.contains(&"task_kind"));
        assert!(spec.node_data_fields.contains(&"pumas_model_ref"));
        assert!(spec.node_data_fields.contains(&"resolved_model_source"));
        assert!(spec.node_data_fields.contains(&"runtime_hint"));
        assert!(spec.node_data_fields.contains(&"generation_options"));
        assert!(spec.node_data_fields.contains(&"task_options"));
        assert!(spec.node_data_fields.contains(&"migration_diagnostics"));
        assert!(
            !spec.diagnostic_code.trim().is_empty(),
            "{} must define migration diagnostics",
            spec.legacy_node_type
        );
    }
}

#[test]
fn legacy_inference_migration_inventory_maps_model_sources_and_task_options() {
    let specs = legacy_inference_node_migration_specs();
    let model_source_nodes = [
        "ollama-inference",
        "llamacpp-inference",
        "pytorch-inference",
        "embedding",
        "reranker",
    ];

    for node_type in model_source_nodes {
        let spec = specs
            .iter()
            .find(|spec| spec.legacy_node_type == node_type)
            .expect("migration spec");
        assert!(
            spec.ports.iter().any(|port| matches!(
                port.action,
                LegacyInferencePortAction::PromoteToNodeData { field_path }
                    if field_path == "pumas_model_ref"
            )),
            "{node_type} must map legacy model source ports to pumas_model_ref"
        );
    }

    let embedding = specs
        .iter()
        .find(|spec| spec.legacy_node_type == "embedding")
        .expect("embedding spec");
    assert_eq!(embedding.task_kind.as_str(), "embedding");
    assert!(embedding.ports.iter().any(|port| {
        port.legacy_port_id == "embedding"
            && matches!(port.action, LegacyInferencePortAction::Preserve)
    }));

    let reranker = specs
        .iter()
        .find(|spec| spec.legacy_node_type == "reranker")
        .expect("reranker spec");
    assert_eq!(reranker.task_kind.as_str(), "rerank");
    assert!(reranker.ports.iter().any(|port| matches!(
        port.action,
        LegacyInferencePortAction::PromoteToNodeData { field_path }
            if field_path == "task_options.top_k"
    )));
}

#[test]
fn legacy_inference_port_mappings_are_deterministic_per_node() {
    for spec in legacy_inference_node_migration_specs() {
        let mut seen = HashSet::new();
        for port in spec.ports {
            let key = (port.direction, port.legacy_port_id);
            assert!(
                seen.insert(key),
                "{} has duplicate migration mapping for {:?} {}",
                spec.legacy_node_type,
                port.direction,
                port.legacy_port_id
            );
        }
    }
}

#[test]
fn canonicalize_workflow_graph_removes_retired_inference_node_types() {
    let registry = NodeRegistry::new();
    let retired_node_types = [
        "ollama-inference",
        "llamacpp-inference",
        "pytorch-inference",
        "embedding",
        "reranker",
    ];
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "ollama".to_string(),
                node_type: "ollama-inference".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "model": "llama3", "prompt": "hello" }),
            },
            GraphNode {
                id: "llamacpp".to_string(),
                node_type: "llamacpp-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({ "model_path": "/models/chat.gguf", "prompt": "hello" }),
            },
            GraphNode {
                id: "pytorch".to_string(),
                node_type: "pytorch-inference".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({ "model_path": "/models/hf", "prompt": "hello" }),
            },
            GraphNode {
                id: "embedding".to_string(),
                node_type: "embedding".to_string(),
                position: super::super::types::Position { x: 300.0, y: 0.0 },
                data: json!({ "model": "embed.gguf", "text": "hello" }),
            },
            GraphNode {
                id: "reranker".to_string(),
                node_type: "reranker".to_string(),
                position: super::super::types::Position { x: 400.0, y: 0.0 },
                data: json!({ "model_path": "/models/rerank.gguf", "query": "hello" }),
            },
        ],
        edges: Vec::new(),
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical_node_types = result
        .graph
        .nodes
        .iter()
        .map(|node| node.node_type.as_str())
        .collect::<HashSet<_>>();

    for retired_node_type in retired_node_types {
        assert!(
            !canonical_node_types.contains(retired_node_type),
            "{retired_node_type} survived canonicalization"
        );
    }
    assert!(result
        .graph
        .nodes
        .iter()
        .all(|node| node.node_type == "llm-inference"));

    let migrated_node_types = result
        .migration_records
        .iter()
        .map(|record| record.node_type.as_str())
        .collect::<HashSet<_>>();
    for retired_node_type in retired_node_types {
        assert!(
            migrated_node_types.contains(retired_node_type),
            "{retired_node_type} missing migration record"
        );
    }
}

#[test]
fn canonicalize_workflow_graph_migrates_legacy_llamacpp_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "model-path".to_string(),
                node_type: "text-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "text": "/models/example.gguf" }),
            },
            GraphNode {
                id: "temperature".to_string(),
                node_type: "number-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 100.0 },
                data: json!({ "value": 0.4 }),
            },
            GraphNode {
                id: "llamacpp".to_string(),
                node_type: "llamacpp-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model_path": "/models/example.gguf",
                    "temperature": 0.4,
                    "max_tokens": 96,
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "model-ref-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 100.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "model-path-llamacpp-model-path".to_string(),
                source: "model-path".to_string(),
                source_handle: "text".to_string(),
                target: "llamacpp".to_string(),
                target_handle: "model_path".to_string(),
            },
            GraphEdge {
                id: "temperature-llamacpp-temperature".to_string(),
                source: "temperature".to_string(),
                source_handle: "value".to_string(),
                target: "llamacpp".to_string(),
                target_handle: "temperature".to_string(),
            },
            GraphEdge {
                id: "llamacpp-response-output-text".to_string(),
                source: "llamacpp".to_string(),
                source_handle: "response".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "llamacpp-model-path-output-text".to_string(),
                source: "llamacpp".to_string(),
                source_handle: "model_path".to_string(),
                target: "model-ref-output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let migrated = canonical
        .nodes
        .iter()
        .find(|node| node.id == "llamacpp")
        .expect("migrated llama.cpp node");

    assert_eq!(migrated.node_type, "llm-inference");
    assert_eq!(migrated.data["task_kind"], json!("text_generation"));
    assert_eq!(migrated.data["runtime_hint"], json!("llamacpp"));
    assert_eq!(
        migrated.data["pumas_model_ref"]["legacy_model_path"],
        json!("/models/example.gguf")
    );
    assert_eq!(
        migrated.data["generation_options"]["sampling"]["temperature"],
        json!(0.4)
    );
    assert_eq!(
        migrated.data["generation_options"]["length"]["max_new_tokens"],
        json!(96)
    );
    assert_eq!(
        migrated.data["migration_diagnostics"][0]["code"],
        json!("legacy_llamacpp_inference_node")
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "model-path-llamacpp-model-path"
            && edge.target == "llamacpp"
            && edge.target_handle == "pumas_model_ref"
    }));
    assert!(!canonical
        .edges
        .iter()
        .any(|edge| edge.target == "llamacpp" && edge.target_handle == "temperature"));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "llamacpp-model-path-output-text"
            && edge.source == "llamacpp"
            && edge.source_handle == "model_ref"
    }));

    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "llamacpp-inference");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert_eq!(
        record.diagnostics_lineage,
        DiagnosticsLineagePolicy::RejectToAvoidSilentChange
    );
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortIdChanged { from, to, kind, .. }
            if from.as_str() == "model_path"
                && to.as_str() == "pumas_model_ref"
                && *kind == PortKind::Input
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "temperature" && *kind == PortKind::Input
    )));
}

#[test]
fn canonicalize_workflow_graph_migrates_legacy_pytorch_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "model-path".to_string(),
                node_type: "text-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "text": "/models/whisper" }),
            },
            GraphNode {
                id: "audio".to_string(),
                node_type: "audio-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 100.0 },
                data: json!({}),
            },
            GraphNode {
                id: "pytorch".to_string(),
                node_type: "pytorch-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model_path": "/models/whisper",
                    "model_type": "asr",
                    "device": "cuda",
                    "temperature": 0.2,
                    "max_tokens": 64,
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "model-path-pytorch-model-path".to_string(),
                source: "model-path".to_string(),
                source_handle: "text".to_string(),
                target: "pytorch".to_string(),
                target_handle: "model_path".to_string(),
            },
            GraphEdge {
                id: "audio-pytorch-audio".to_string(),
                source: "audio".to_string(),
                source_handle: "audio".to_string(),
                target: "pytorch".to_string(),
                target_handle: "audio".to_string(),
            },
            GraphEdge {
                id: "pytorch-response-output-text".to_string(),
                source: "pytorch".to_string(),
                source_handle: "response".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let migrated = canonical
        .nodes
        .iter()
        .find(|node| node.id == "pytorch")
        .expect("migrated PyTorch node");

    assert_eq!(migrated.node_type, "llm-inference");
    assert_eq!(migrated.data["task_kind"], json!("audio_transcription"));
    assert_eq!(migrated.data["runtime_hint"], json!("transformers_pytorch"));
    assert_eq!(
        migrated.data["pumas_model_ref"]["legacy_model_path"],
        json!("/models/whisper")
    );
    assert_eq!(
        migrated.data["resolved_model_source"]["legacy_model_type"],
        json!("asr")
    );
    assert_eq!(
        migrated.data["runtime_hint_details"]["device"],
        json!("cuda")
    );
    assert_eq!(
        migrated.data["generation_options"]["sampling"]["temperature"],
        json!(0.2)
    );
    assert_eq!(
        migrated.data["generation_options"]["length"]["max_new_tokens"],
        json!(64)
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "model-path-pytorch-model-path"
            && edge.target == "pytorch"
            && edge.target_handle == "pumas_model_ref"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "audio-pytorch-audio"
            && edge.target == "pytorch"
            && edge.target_handle == "audio"
    }));

    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "pytorch-inference");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortIdChanged { from, to, kind, .. }
            if from.as_str() == "model_path"
                && to.as_str() == "pumas_model_ref"
                && *kind == PortKind::Input
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model_type" && *kind == PortKind::Input
    )));
}

#[test]
fn canonicalize_workflow_graph_migrates_legacy_embedding_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "text".to_string(),
                node_type: "text-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "text": "Pantograph embeddings are inference tasks." }),
            },
            GraphNode {
                id: "embedding".to_string(),
                node_type: "embedding".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model": "bge-small-en-v1.5",
                }),
            },
            GraphNode {
                id: "vector-output".to_string(),
                node_type: "vector-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "metadata-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 100.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "text-embedding-text".to_string(),
                source: "text".to_string(),
                source_handle: "text".to_string(),
                target: "embedding".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "embedding-vector-output-vector".to_string(),
                source: "embedding".to_string(),
                source_handle: "embedding".to_string(),
                target: "vector-output".to_string(),
                target_handle: "vector".to_string(),
            },
            GraphEdge {
                id: "embedding-metadata-output-text".to_string(),
                source: "embedding".to_string(),
                source_handle: "metadata".to_string(),
                target: "metadata-output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let migrated = canonical
        .nodes
        .iter()
        .find(|node| node.id == "embedding")
        .expect("migrated embedding node");

    assert_eq!(migrated.node_type, "llm-inference");
    assert_eq!(migrated.data["task_kind"], json!("embedding"));
    assert_eq!(migrated.data["runtime_hint"], json!("llamacpp"));
    assert_eq!(
        migrated.data["pumas_model_ref"]["legacy_model"],
        json!("bge-small-en-v1.5")
    );
    assert_eq!(
        migrated.data["migration_diagnostics"][0]["code"],
        json!("legacy_embedding_node")
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "text-embedding-text"
            && edge.target == "embedding"
            && edge.target_handle == "text"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "embedding-vector-output-vector"
            && edge.source == "embedding"
            && edge.source_handle == "embedding"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "embedding-metadata-output-text"
            && edge.source == "embedding"
            && edge.source_handle == "metadata"
    }));

    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "embedding");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::NodeTypeChanged { from, to, .. }
            if from.as_str() == "embedding" && to.as_str() == "llm-inference"
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model" && *kind == PortKind::Input
    )));
}

#[test]
fn canonicalize_workflow_graph_migrates_legacy_reranker_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "query".to_string(),
                node_type: "text-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "text": "Which document is most relevant?" }),
            },
            GraphNode {
                id: "documents".to_string(),
                node_type: "text-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 100.0 },
                data: json!({ "text": "[\"a\", \"b\"]" }),
            },
            GraphNode {
                id: "reranker".to_string(),
                node_type: "reranker".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model_path": "/models/reranker.gguf",
                    "top_k": 2,
                    "return_documents": true,
                }),
            },
            GraphNode {
                id: "results-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "top-document-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 100.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "query-reranker-query".to_string(),
                source: "query".to_string(),
                source_handle: "text".to_string(),
                target: "reranker".to_string(),
                target_handle: "query".to_string(),
            },
            GraphEdge {
                id: "documents-reranker-documents-json".to_string(),
                source: "documents".to_string(),
                source_handle: "text".to_string(),
                target: "reranker".to_string(),
                target_handle: "documents_json".to_string(),
            },
            GraphEdge {
                id: "reranker-results-output-text".to_string(),
                source: "reranker".to_string(),
                source_handle: "results".to_string(),
                target: "results-output".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "reranker-top-document-output-text".to_string(),
                source: "reranker".to_string(),
                source_handle: "top_document".to_string(),
                target: "top-document-output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let migrated = canonical
        .nodes
        .iter()
        .find(|node| node.id == "reranker")
        .expect("migrated reranker node");

    assert_eq!(migrated.node_type, "llm-inference");
    assert_eq!(migrated.data["task_kind"], json!("rerank"));
    assert_eq!(migrated.data["runtime_hint"], json!("llamacpp"));
    assert_eq!(
        migrated.data["pumas_model_ref"]["legacy_model_path"],
        json!("/models/reranker.gguf")
    );
    assert_eq!(migrated.data["task_options"]["top_k"], json!(2));
    assert_eq!(
        migrated.data["task_options"]["return_documents"],
        json!(true)
    );
    assert_eq!(
        migrated.data["migration_diagnostics"][0]["code"],
        json!("legacy_reranker_node")
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "query-reranker-query"
            && edge.target == "reranker"
            && edge.target_handle == "query"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "documents-reranker-documents-json"
            && edge.target == "reranker"
            && edge.target_handle == "documents_json"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "reranker-results-output-text"
            && edge.source == "reranker"
            && edge.source_handle == "results"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "reranker-top-document-output-text"
            && edge.source == "reranker"
            && edge.source_handle == "top_document"
    }));

    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "reranker");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::NodeTypeChanged { from, to, .. }
            if from.as_str() == "reranker" && to.as_str() == "llm-inference"
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model_path" && *kind == PortKind::Input
    )));
}

#[test]
fn canonicalize_workflow_graph_preserves_mixed_inference_topology() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "text": "Summarize the candidates." }),
            },
            GraphNode {
                id: "llama".to_string(),
                node_type: "llamacpp-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model_path": "/models/text.gguf",
                    "max_tokens": 128,
                }),
            },
            GraphNode {
                id: "embedding".to_string(),
                node_type: "embedding".to_string(),
                position: super::super::types::Position { x: 100.0, y: 120.0 },
                data: json!({
                    "model": "bge-small",
                }),
            },
            GraphNode {
                id: "rerank".to_string(),
                node_type: "reranker".to_string(),
                position: super::super::types::Position { x: 220.0, y: 0.0 },
                data: json!({
                    "model_path": "/models/rerank.gguf",
                    "top_k": 1,
                }),
            },
            GraphNode {
                id: "text-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 340.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "vector-output".to_string(),
                node_type: "vector-output".to_string(),
                position: super::super::types::Position { x: 340.0, y: 120.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "prompt-llama-prompt".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "llama".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "prompt-embedding-text".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "embedding".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "llama-response-rerank-documents".to_string(),
                source: "llama".to_string(),
                source_handle: "response".to_string(),
                target: "rerank".to_string(),
                target_handle: "documents_json".to_string(),
            },
            GraphEdge {
                id: "rerank-results-output-text".to_string(),
                source: "rerank".to_string(),
                source_handle: "results".to_string(),
                target: "text-output".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "embedding-vector-output-vector".to_string(),
                source: "embedding".to_string(),
                source_handle: "embedding".to_string(),
                target: "vector-output".to_string(),
                target_handle: "vector".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;

    for node_id in ["llama", "embedding", "rerank"] {
        let node = canonical
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .expect("migrated inference node");
        assert_eq!(node.node_type, "llm-inference");
    }
    assert_eq!(
        canonical
            .nodes
            .iter()
            .find(|node| node.id == "llama")
            .expect("migrated llama node")
            .data["generation_options"]["length"]["max_new_tokens"],
        json!(128)
    );
    assert_eq!(
        canonical
            .nodes
            .iter()
            .find(|node| node.id == "rerank")
            .expect("migrated rerank node")
            .data["task_options"]["top_k"],
        json!(1)
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "llama-response-rerank-documents"
            && edge.source == "llama"
            && edge.source_handle == "response"
            && edge.target == "rerank"
            && edge.target_handle == "documents_json"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "rerank-results-output-text"
            && edge.source == "rerank"
            && edge.source_handle == "results"
            && edge.target == "text-output"
            && edge.target_handle == "text"
    }));
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "embedding-vector-output-vector"
            && edge.source == "embedding"
            && edge.source_handle == "embedding"
            && edge.target == "vector-output"
            && edge.target_handle == "vector"
    }));

    let migrated_node_ids = result
        .migration_records
        .iter()
        .flat_map(|record| record.changes.iter())
        .filter_map(|change| match change {
            ContractUpgradeChange::NodeTypeChanged { node_id, .. } => Some(node_id.as_str()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    assert_eq!(
        migrated_node_ids,
        HashSet::from(["embedding", "llama", "rerank"])
    );
}

#[test]
fn canonicalize_workflow_graph_hydrates_expand_settings_and_passthrough_edges() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "source".to_string(),
                node_type: "model-provider".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({
                    "inference_settings": [
                        {
                            "key": "steps",
                            "label": "Steps",
                            "param_type": "Number",
                            "default": 30,
                        }
                    ]
                }),
            },
            GraphNode {
                id: "expand".to_string(),
                node_type: "expand-settings".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "diffusion".to_string(),
                node_type: "diffusion-inference".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "source-settings-expand-settings".to_string(),
                source: "source".to_string(),
                source_handle: "inference_settings".to_string(),
                target: "expand".to_string(),
                target_handle: "inference_settings".to_string(),
            },
            GraphEdge {
                id: "expand-settings-diffusion-settings".to_string(),
                source: "expand".to_string(),
                source_handle: "inference_settings".to_string(),
                target: "diffusion".to_string(),
                target_handle: "inference_settings".to_string(),
            },
        ],
        derived_graph: None,
    };

    let canonical = canonicalize_workflow_graph(graph, &registry);
    let expand_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "expand")
        .expect("expand node");
    let diffusion_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "diffusion")
        .expect("diffusion node");
    let expand_outputs = expand_node.data["definition"]["outputs"]
        .as_array()
        .expect("expand outputs");
    let diffusion_inputs = diffusion_node.data["definition"]["inputs"]
        .as_array()
        .expect("diffusion inputs");

    assert!(expand_outputs
        .iter()
        .any(|port| port["id"] == json!("steps")));
    assert!(diffusion_inputs
        .iter()
        .any(|port| port["id"] == json!("steps")));
    assert!(canonical.edges.iter().any(|edge| {
        edge.source == "expand"
            && edge.source_handle == "steps"
            && edge.target == "diffusion"
            && edge.target_handle == "steps"
    }));
}
