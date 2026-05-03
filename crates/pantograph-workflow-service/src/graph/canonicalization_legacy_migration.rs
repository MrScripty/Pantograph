use std::collections::HashMap;

use pantograph_node_contracts::{
    ContractUpgradeChange, ContractUpgradeDiagnostic, ContractUpgradeOutcome,
    ContractUpgradeRecord, ContractUpgradeRejectionReason, DiagnosticsLineagePolicy,
    NodeInstanceId, NodeTypeId, PortId, PortKind,
};
use serde_json::{json, Map, Value};

use super::super::types::WorkflowGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LegacyNodeMigrationKind {
    SystemPrompt,
    OllamaInference,
    LlamaCppInference,
    PyTorchInference,
    Embedding,
    Reranker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CanonicalInferenceTaskKind {
    TextGeneration,
    AudioTranscription,
    Embedding,
    Rerank,
}

impl CanonicalInferenceTaskKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::TextGeneration => "text_generation",
            Self::AudioTranscription => "audio_transcription",
            Self::Embedding => "embedding",
            Self::Rerank => "rerank",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CanonicalInferenceRuntimeHint {
    RetiredOllama,
    LlamaCpp,
    TransformersPyTorch,
    OpenAiCompatible,
}

impl CanonicalInferenceRuntimeHint {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::RetiredOllama => "retired_ollama",
            Self::LlamaCpp => "llamacpp",
            Self::TransformersPyTorch => "transformers_pytorch",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum LegacyInferencePortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LegacyInferencePortAction {
    Preserve,
    PromoteToNodeData { field_path: &'static str },
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LegacyInferencePortMigration {
    pub(super) direction: LegacyInferencePortDirection,
    pub(super) legacy_port_id: &'static str,
    pub(super) action: LegacyInferencePortAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LegacyInferenceNodeMigrationSpec {
    pub(super) legacy_node_type: &'static str,
    pub(super) canonical_node_type: &'static str,
    pub(super) task_kind: CanonicalInferenceTaskKind,
    pub(super) runtime_hint: CanonicalInferenceRuntimeHint,
    pub(super) node_data_fields: &'static [&'static str],
    pub(super) ports: &'static [LegacyInferencePortMigration],
    pub(super) diagnostic_code: &'static str,
}

const CANONICAL_INFERENCE_NODE_TYPE: &str = "llm-inference";
const CANONICAL_INFERENCE_NODE_DATA_FIELDS: &[&str] = &[
    "task_kind",
    "pumas_model_ref",
    "resolved_model_source",
    "runtime_hint",
    "generation_options",
    "task_options",
    "migration_diagnostics",
];

const OLLAMA_PORTS: &[LegacyInferencePortMigration] = &[
    legacy_input_preserve("prompt"),
    legacy_input_preserve("system_prompt"),
    legacy_input_data("model", "pumas_model_ref"),
    legacy_input_data("temperature", "generation_options.temperature"),
    legacy_input_data("max_tokens", "generation_options.max_new_tokens"),
    legacy_input_data("inference_settings", "generation_options"),
    legacy_output_preserve("response"),
    legacy_output_remove("model_used"),
    legacy_output_data("model_ref", "pumas_model_ref"),
    legacy_output_preserve("stream"),
];

const LLAMACPP_PORTS: &[LegacyInferencePortMigration] = &[
    legacy_input_data("model_path", "pumas_model_ref"),
    legacy_input_preserve("prompt"),
    legacy_input_preserve("system_prompt"),
    legacy_input_data("temperature", "generation_options.temperature"),
    legacy_input_data("max_tokens", "generation_options.max_new_tokens"),
    legacy_input_preserve("tools"),
    legacy_input_preserve("kv_cache_in"),
    legacy_input_data("inference_settings", "generation_options"),
    legacy_output_preserve("response"),
    legacy_output_data("model_path", "pumas_model_ref"),
    legacy_output_data("model_ref", "pumas_model_ref"),
    legacy_output_preserve("tool_calls"),
    legacy_output_preserve("has_tool_calls"),
    legacy_output_preserve("kv_cache_out"),
    legacy_output_preserve("stream"),
];

const PYTORCH_PORTS: &[LegacyInferencePortMigration] = &[
    legacy_input_data("model_path", "pumas_model_ref"),
    legacy_input_preserve("prompt"),
    legacy_input_preserve("audio"),
    legacy_input_preserve("system_prompt"),
    legacy_input_data("temperature", "generation_options.temperature"),
    legacy_input_data("max_tokens", "generation_options.max_new_tokens"),
    legacy_input_data("device", "runtime_hint.device"),
    legacy_input_data("model_type", "resolved_model_source.model_type"),
    legacy_input_preserve("kv_cache_in"),
    legacy_input_data("inference_settings", "generation_options"),
    legacy_input_data("environment_ref", "runtime_hint.environment_ref"),
    legacy_output_preserve("response"),
    legacy_output_data("model_ref", "pumas_model_ref"),
    legacy_output_preserve("kv_cache_out"),
    legacy_output_preserve("stream"),
];

const EMBEDDING_PORTS: &[LegacyInferencePortMigration] = &[
    legacy_input_preserve("text"),
    legacy_input_data("model", "pumas_model_ref"),
    legacy_output_preserve("embedding"),
    legacy_output_preserve("metadata"),
];

const RERANKER_PORTS: &[LegacyInferencePortMigration] = &[
    legacy_input_data("model_path", "pumas_model_ref"),
    legacy_input_preserve("query"),
    legacy_input_preserve("documents"),
    legacy_input_preserve("documents_json"),
    legacy_input_data("top_k", "task_options.top_k"),
    legacy_input_data("return_documents", "task_options.return_documents"),
    legacy_input_data("inference_settings", "task_options"),
    legacy_output_preserve("results"),
    legacy_output_preserve("scores"),
    legacy_output_preserve("top_document"),
    legacy_output_preserve("top_score"),
    legacy_output_data("model_path", "pumas_model_ref"),
    legacy_output_data("model_ref", "pumas_model_ref"),
];

const GENERIC_INFERENCE_PORTS: &[LegacyInferencePortMigration] = &[
    legacy_input_preserve("prompt"),
    legacy_input_preserve("system_prompt"),
    legacy_input_preserve("context"),
    legacy_input_preserve("tools"),
    legacy_input_preserve("kv_cache_in"),
    legacy_input_data("inference_settings", "generation_options"),
    legacy_output_preserve("response"),
    legacy_output_preserve("tool_calls"),
    legacy_output_preserve("has_tool_calls"),
    legacy_output_preserve("kv_cache_out"),
    legacy_output_preserve("stream"),
];

const LEGACY_INFERENCE_NODE_MIGRATION_SPECS: &[LegacyInferenceNodeMigrationSpec] = &[
    LegacyInferenceNodeMigrationSpec {
        legacy_node_type: "ollama-inference",
        canonical_node_type: CANONICAL_INFERENCE_NODE_TYPE,
        task_kind: CanonicalInferenceTaskKind::TextGeneration,
        runtime_hint: CanonicalInferenceRuntimeHint::RetiredOllama,
        node_data_fields: CANONICAL_INFERENCE_NODE_DATA_FIELDS,
        ports: OLLAMA_PORTS,
        diagnostic_code: "legacy_ollama_backend_retired",
    },
    LegacyInferenceNodeMigrationSpec {
        legacy_node_type: "llamacpp-inference",
        canonical_node_type: CANONICAL_INFERENCE_NODE_TYPE,
        task_kind: CanonicalInferenceTaskKind::TextGeneration,
        runtime_hint: CanonicalInferenceRuntimeHint::LlamaCpp,
        node_data_fields: CANONICAL_INFERENCE_NODE_DATA_FIELDS,
        ports: LLAMACPP_PORTS,
        diagnostic_code: "legacy_llamacpp_inference_node",
    },
    LegacyInferenceNodeMigrationSpec {
        legacy_node_type: "pytorch-inference",
        canonical_node_type: CANONICAL_INFERENCE_NODE_TYPE,
        task_kind: CanonicalInferenceTaskKind::TextGeneration,
        runtime_hint: CanonicalInferenceRuntimeHint::TransformersPyTorch,
        node_data_fields: CANONICAL_INFERENCE_NODE_DATA_FIELDS,
        ports: PYTORCH_PORTS,
        diagnostic_code: "legacy_pytorch_inference_node",
    },
    LegacyInferenceNodeMigrationSpec {
        legacy_node_type: "embedding",
        canonical_node_type: CANONICAL_INFERENCE_NODE_TYPE,
        task_kind: CanonicalInferenceTaskKind::Embedding,
        runtime_hint: CanonicalInferenceRuntimeHint::LlamaCpp,
        node_data_fields: CANONICAL_INFERENCE_NODE_DATA_FIELDS,
        ports: EMBEDDING_PORTS,
        diagnostic_code: "legacy_embedding_node",
    },
    LegacyInferenceNodeMigrationSpec {
        legacy_node_type: "reranker",
        canonical_node_type: CANONICAL_INFERENCE_NODE_TYPE,
        task_kind: CanonicalInferenceTaskKind::Rerank,
        runtime_hint: CanonicalInferenceRuntimeHint::LlamaCpp,
        node_data_fields: CANONICAL_INFERENCE_NODE_DATA_FIELDS,
        ports: RERANKER_PORTS,
        diagnostic_code: "legacy_reranker_node",
    },
    LegacyInferenceNodeMigrationSpec {
        legacy_node_type: "llm-inference",
        canonical_node_type: CANONICAL_INFERENCE_NODE_TYPE,
        task_kind: CanonicalInferenceTaskKind::TextGeneration,
        runtime_hint: CanonicalInferenceRuntimeHint::OpenAiCompatible,
        node_data_fields: CANONICAL_INFERENCE_NODE_DATA_FIELDS,
        ports: GENERIC_INFERENCE_PORTS,
        diagnostic_code: "legacy_generic_inference_node",
    },
];

pub(super) fn legacy_inference_node_migration_specs() -> &'static [LegacyInferenceNodeMigrationSpec]
{
    LEGACY_INFERENCE_NODE_MIGRATION_SPECS
}

const fn legacy_input_preserve(legacy_port_id: &'static str) -> LegacyInferencePortMigration {
    LegacyInferencePortMigration {
        direction: LegacyInferencePortDirection::Input,
        legacy_port_id,
        action: LegacyInferencePortAction::Preserve,
    }
}

const fn legacy_output_preserve(legacy_port_id: &'static str) -> LegacyInferencePortMigration {
    LegacyInferencePortMigration {
        direction: LegacyInferencePortDirection::Output,
        legacy_port_id,
        action: LegacyInferencePortAction::Preserve,
    }
}

const fn legacy_input_data(
    legacy_port_id: &'static str,
    field_path: &'static str,
) -> LegacyInferencePortMigration {
    LegacyInferencePortMigration {
        direction: LegacyInferencePortDirection::Input,
        legacy_port_id,
        action: LegacyInferencePortAction::PromoteToNodeData { field_path },
    }
}

const fn legacy_output_data(
    legacy_port_id: &'static str,
    field_path: &'static str,
) -> LegacyInferencePortMigration {
    LegacyInferencePortMigration {
        direction: LegacyInferencePortDirection::Output,
        legacy_port_id,
        action: LegacyInferencePortAction::PromoteToNodeData { field_path },
    }
}

const fn legacy_output_remove(legacy_port_id: &'static str) -> LegacyInferencePortMigration {
    LegacyInferencePortMigration {
        direction: LegacyInferencePortDirection::Output,
        legacy_port_id,
        action: LegacyInferencePortAction::Remove,
    }
}

pub(super) fn canonicalize_legacy_node_types(
    graph: WorkflowGraph,
) -> (WorkflowGraph, HashMap<String, LegacyNodeMigrationKind>) {
    let mut migrated_nodes = HashMap::new();
    let nodes = graph
        .nodes
        .into_iter()
        .map(|mut node| {
            match node.node_type.as_str() {
                "system-prompt" => {
                    migrated_nodes.insert(node.id.clone(), LegacyNodeMigrationKind::SystemPrompt);
                    node.node_type = "text-input".to_string();
                    if let Some(data) = node.data.as_object_mut() {
                        if let Some(prompt) = data.remove("prompt") {
                            data.entry("text".to_string()).or_insert(prompt);
                        }
                    }
                }
                "ollama-inference" => {
                    migrated_nodes
                        .insert(node.id.clone(), LegacyNodeMigrationKind::OllamaInference);
                    node.node_type = "llm-inference".to_string();
                    migrate_ollama_node_data(&mut node.data);
                }
                "llamacpp-inference" => {
                    migrated_nodes
                        .insert(node.id.clone(), LegacyNodeMigrationKind::LlamaCppInference);
                    node.node_type = "llm-inference".to_string();
                    migrate_llamacpp_node_data(&mut node.data);
                }
                "pytorch-inference" => {
                    migrated_nodes
                        .insert(node.id.clone(), LegacyNodeMigrationKind::PyTorchInference);
                    node.node_type = "llm-inference".to_string();
                    migrate_pytorch_node_data(&mut node.data);
                }
                "embedding" => {
                    migrated_nodes.insert(node.id.clone(), LegacyNodeMigrationKind::Embedding);
                    node.node_type = "llm-inference".to_string();
                    migrate_embedding_node_data(&mut node.data);
                }
                "reranker" => {
                    migrated_nodes.insert(node.id.clone(), LegacyNodeMigrationKind::Reranker);
                    node.node_type = "llm-inference".to_string();
                    migrate_reranker_node_data(&mut node.data);
                }
                _ => {}
            }

            node
        })
        .collect::<Vec<_>>();
    let edges = graph
        .edges
        .into_iter()
        .filter_map(|edge| {
            if migrated_nodes.get(&edge.source) == Some(&LegacyNodeMigrationKind::OllamaInference)
                && matches!(edge.source_handle.as_str(), "model_used" | "model_ref")
            {
                return None;
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::OllamaInference)
                && matches!(
                    edge.target_handle.as_str(),
                    "model" | "temperature" | "max_tokens"
                )
            {
                return None;
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::LlamaCppInference)
                && matches!(edge.target_handle.as_str(), "temperature" | "max_tokens")
            {
                return None;
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::PyTorchInference)
                && matches!(
                    edge.target_handle.as_str(),
                    "temperature" | "max_tokens" | "device" | "model_type" | "environment_ref"
                )
            {
                return None;
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::Embedding)
                && edge.target_handle == "model"
            {
                return None;
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::Reranker)
                && matches!(
                    edge.target_handle.as_str(),
                    "model_path" | "top_k" | "return_documents"
                )
            {
                return None;
            }
            if migrated_nodes.get(&edge.source) == Some(&LegacyNodeMigrationKind::Reranker)
                && matches!(edge.source_handle.as_str(), "model_path" | "model_ref")
            {
                return None;
            }
            Some(edge)
        })
        .map(|mut edge| {
            if migrated_nodes.get(&edge.source) == Some(&LegacyNodeMigrationKind::SystemPrompt)
                && edge.source_handle == "prompt"
            {
                edge.source_handle = "text".to_string();
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::SystemPrompt)
                && edge.target_handle == "prompt"
            {
                edge.target_handle = "text".to_string();
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::LlamaCppInference)
                && edge.target_handle == "model_path"
            {
                edge.target_handle = "pumas_model_ref".to_string();
            }
            if migrated_nodes.get(&edge.source) == Some(&LegacyNodeMigrationKind::LlamaCppInference)
                && edge.source_handle == "model_path"
            {
                edge.source_handle = "model_ref".to_string();
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::PyTorchInference)
                && edge.target_handle == "model_path"
            {
                edge.target_handle = "pumas_model_ref".to_string();
            }
            edge
        })
        .collect::<Vec<_>>();
    (
        WorkflowGraph {
            nodes,
            edges,
            derived_graph: None,
        },
        migrated_nodes,
    )
}

pub(super) fn legacy_node_type_migration_records(
    migrated_nodes: &HashMap<String, LegacyNodeMigrationKind>,
) -> Vec<ContractUpgradeRecord> {
    let mut records = migrated_nodes
        .iter()
        .filter_map(|(node_id, migration)| match migration {
            LegacyNodeMigrationKind::SystemPrompt => legacy_system_prompt_migration_record(node_id),
            LegacyNodeMigrationKind::OllamaInference => legacy_ollama_migration_record(node_id),
            LegacyNodeMigrationKind::LlamaCppInference => legacy_llamacpp_migration_record(node_id),
            LegacyNodeMigrationKind::PyTorchInference => legacy_pytorch_migration_record(node_id),
            LegacyNodeMigrationKind::Embedding => legacy_embedding_migration_record(node_id),
            LegacyNodeMigrationKind::Reranker => legacy_reranker_migration_record(node_id),
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        let left_node = upgrade_record_node_id(left);
        let right_node = upgrade_record_node_id(right);
        left_node.cmp(&right_node)
    });
    records
}

fn migrate_ollama_node_data(data: &mut serde_json::Value) {
    if !data.is_object() {
        *data = json!({});
    }

    let Some(object) = data.as_object_mut() else {
        return;
    };
    let legacy_model = object.get("model").cloned();
    let legacy_temperature = object.get("temperature").cloned();
    let legacy_max_tokens = object.get("max_tokens").cloned();

    object
        .entry("task_kind".to_string())
        .or_insert_with(|| json!("text_generation"));
    object
        .entry("runtime_hint".to_string())
        .or_insert_with(|| json!("retired_ollama"));
    object
        .entry("pumas_model_ref".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "source": "legacy_ollama",
                "legacy_model": legacy_model,
                "message": "Ollama is retired as a first-party Pantograph backend; select a Pumas model reference before running this node."
            })
        });
    object
        .entry("migration_diagnostics".to_string())
        .or_insert_with(|| {
            json!([{
                "code": "legacy_ollama_backend_retired",
                "severity": "error",
                "message": "Migrated from ollama-inference to llm-inference without preserving Ollama execution support. Select a Pumas model reference and supported runtime before execution.",
                "legacy_model": legacy_model,
                "legacy_temperature": legacy_temperature,
                "legacy_max_tokens": legacy_max_tokens
            }])
        });
}

fn migrate_llamacpp_node_data(data: &mut serde_json::Value) {
    let object = ensure_json_object(data);
    let legacy_model_path = object.get("model_path").cloned();
    let legacy_temperature = object.get("temperature").cloned();
    let legacy_max_tokens = object.get("max_tokens").cloned();

    object
        .entry("task_kind".to_string())
        .or_insert_with(|| json!("text_generation"));
    object
        .entry("runtime_hint".to_string())
        .or_insert_with(|| json!("llamacpp"));
    object
        .entry("pumas_model_ref".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "source": "legacy_llamacpp",
                "legacy_model_path": legacy_model_path,
                "message": "Resolve this legacy llama.cpp model path through Pumas before running the canonical inference node."
            })
        });

    let generation_options = object
        .entry("generation_options".to_string())
        .or_insert_with(|| json!({}));
    if let Some(options) = generation_options.as_object_mut() {
        if let Some(value) = legacy_temperature.clone() {
            options.entry("temperature".to_string()).or_insert(value);
        }
        if let Some(value) = legacy_max_tokens.clone() {
            options.entry("max_new_tokens".to_string()).or_insert(value);
        }
    }

    object
        .entry("migration_diagnostics".to_string())
        .or_insert_with(|| {
            json!([{
                "code": "legacy_llamacpp_inference_node",
                "severity": "warning",
                "message": "Migrated from llamacpp-inference to canonical llm-inference. The legacy model path was retained as unresolved Pumas model-reference evidence until Pumas resolves it.",
                "legacy_model_path": legacy_model_path,
                "legacy_temperature": legacy_temperature,
                "legacy_max_tokens": legacy_max_tokens
            }])
        });
}

fn migrate_pytorch_node_data(data: &mut serde_json::Value) {
    let object = ensure_json_object(data);
    let legacy_model_path = object.get("model_path").cloned();
    let legacy_temperature = object.get("temperature").cloned();
    let legacy_max_tokens = object.get("max_tokens").cloned();
    let legacy_device = object.get("device").cloned();
    let legacy_model_type = object.get("model_type").cloned();
    let legacy_environment_ref = object.get("environment_ref").cloned();
    let task_kind = pytorch_task_kind_from_model_type(legacy_model_type.as_ref());

    object
        .entry("task_kind".to_string())
        .or_insert_with(|| json!(task_kind.as_str()));
    object
        .entry("runtime_hint".to_string())
        .or_insert_with(|| json!("transformers_pytorch"));
    object
        .entry("pumas_model_ref".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "source": "legacy_pytorch",
                "legacy_model_path": legacy_model_path,
                "message": "Resolve this legacy PyTorch/HF model source through Pumas before running the canonical inference node."
            })
        });

    let generation_options = object
        .entry("generation_options".to_string())
        .or_insert_with(|| json!({}));
    if let Some(options) = generation_options.as_object_mut() {
        if let Some(value) = legacy_temperature.clone() {
            options.entry("temperature".to_string()).or_insert(value);
        }
        if let Some(value) = legacy_max_tokens.clone() {
            options.entry("max_new_tokens".to_string()).or_insert(value);
        }
    }

    let runtime_hint = object
        .entry("runtime_hint_details".to_string())
        .or_insert_with(|| json!({}));
    if let Some(details) = runtime_hint.as_object_mut() {
        if let Some(value) = legacy_device.clone() {
            details.entry("device".to_string()).or_insert(value);
        }
        if let Some(value) = legacy_environment_ref.clone() {
            details
                .entry("environment_ref".to_string())
                .or_insert(value);
        }
    }

    object
        .entry("resolved_model_source".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "legacy_model_type": legacy_model_type
            })
        });
    object
        .entry("migration_diagnostics".to_string())
        .or_insert_with(|| {
            json!([{
                "code": "legacy_pytorch_inference_node",
                "severity": "warning",
                "message": "Migrated from pytorch-inference to canonical llm-inference. The legacy model path and model type were retained as unresolved Pumas/Transformers evidence until Pumas resolves the package facts.",
                "legacy_model_path": legacy_model_path,
                "legacy_model_type": legacy_model_type,
                "legacy_device": legacy_device,
                "legacy_environment_ref": legacy_environment_ref
            }])
        });
}

fn migrate_embedding_node_data(data: &mut serde_json::Value) {
    let object = ensure_json_object(data);
    let legacy_model = object.get("model").cloned();

    object
        .entry("task_kind".to_string())
        .or_insert_with(|| json!("embedding"));
    object
        .entry("runtime_hint".to_string())
        .or_insert_with(|| json!("llamacpp"));
    object
        .entry("pumas_model_ref".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "source": "legacy_embedding",
                "legacy_model": legacy_model,
                "message": "Resolve this legacy embedding model reference through Pumas before running the canonical inference node."
            })
        });
    object
        .entry("migration_diagnostics".to_string())
        .or_insert_with(|| {
            json!([{
                "code": "legacy_embedding_node",
                "severity": "warning",
                "message": "Migrated from dedicated embedding node to canonical llm-inference with task_kind=embedding. Dedicated embedding runtime residency is now backend-local rather than graph-visible.",
                "legacy_model": legacy_model
            }])
        });
}

fn migrate_reranker_node_data(data: &mut serde_json::Value) {
    let object = ensure_json_object(data);
    let legacy_model_path = object.get("model_path").cloned();
    let legacy_top_k = object.get("top_k").cloned();
    let legacy_return_documents = object.get("return_documents").cloned();

    object
        .entry("task_kind".to_string())
        .or_insert_with(|| json!("rerank"));
    object
        .entry("runtime_hint".to_string())
        .or_insert_with(|| json!("llamacpp"));
    object
        .entry("pumas_model_ref".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "source": "legacy_reranker",
                "legacy_model_path": legacy_model_path,
                "message": "Resolve this legacy GGUF reranker model path through Pumas before running the canonical inference node."
            })
        });

    let task_options = object
        .entry("task_options".to_string())
        .or_insert_with(|| json!({}));
    if let Some(options) = task_options.as_object_mut() {
        if let Some(value) = legacy_top_k.clone() {
            options.entry("top_k".to_string()).or_insert(value);
        }
        if let Some(value) = legacy_return_documents.clone() {
            options
                .entry("return_documents".to_string())
                .or_insert(value);
        }
    }

    object
        .entry("migration_diagnostics".to_string())
        .or_insert_with(|| {
            json!([{
                "code": "legacy_reranker_node",
                "severity": "warning",
                "message": "Migrated from dedicated reranker node to canonical llm-inference with task_kind=rerank. Backend-specific reranker request options are now canonical task options.",
                "legacy_model_path": legacy_model_path,
                "legacy_top_k": legacy_top_k,
                "legacy_return_documents": legacy_return_documents
            }])
        });
}

fn pytorch_task_kind_from_model_type(model_type: Option<&Value>) -> CanonicalInferenceTaskKind {
    let Some(model_type) = model_type.and_then(Value::as_str) else {
        return CanonicalInferenceTaskKind::TextGeneration;
    };
    match model_type.trim().to_ascii_lowercase().as_str() {
        "asr"
        | "automatic-speech-recognition"
        | "automatic_speech_recognition"
        | "audio_transcription"
        | "speech_to_text"
        | "speech-to-text" => CanonicalInferenceTaskKind::AudioTranscription,
        _ => CanonicalInferenceTaskKind::TextGeneration,
    }
}

fn ensure_json_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .expect("value should be an object after normalization")
}

fn legacy_system_prompt_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("system-prompt".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::PreservePrimitiveLineage,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("system-prompt".to_string()).ok()?,
                to: NodeTypeId::try_from("text-input".to_string()).ok()?,
            },
            ContractUpgradeChange::PortIdChanged {
                node_id,
                kind: PortKind::Output,
                from: PortId::try_from("prompt".to_string()).ok()?,
                to: PortId::try_from("text".to_string()).ok()?,
            },
        ],
        diagnostics: Vec::new(),
    };
    record.validate().ok()?;
    Some(record)
}

fn legacy_ollama_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("ollama-inference".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("ollama-inference".to_string()).ok()?,
                to: NodeTypeId::try_from("llm-inference".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("model".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("temperature".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("max_tokens".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                port_id: PortId::try_from("model_used".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                port_id: PortId::try_from("model_ref".to_string()).ok()?,
            },
        ],
        diagnostics: vec![ContractUpgradeDiagnostic {
            reason: ContractUpgradeRejectionReason::UnsupportedLegacyContract,
            message: "Ollama execution is retired; this node was migrated to llm-inference with an unresolved Pumas model reference diagnostic.".to_string(),
            node_id: Some(node_id),
            node_type: Some(NodeTypeId::try_from("ollama-inference".to_string()).ok()?),
            port_id: None,
        }],
    };
    record.validate().ok()?;
    Some(record)
}

fn legacy_llamacpp_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("llamacpp-inference".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("llamacpp-inference".to_string()).ok()?,
                to: NodeTypeId::try_from("llm-inference".to_string()).ok()?,
            },
            ContractUpgradeChange::PortIdChanged {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                from: PortId::try_from("model_path".to_string()).ok()?,
                to: PortId::try_from("pumas_model_ref".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("temperature".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("max_tokens".to_string()).ok()?,
            },
            ContractUpgradeChange::PortIdChanged {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                from: PortId::try_from("model_path".to_string()).ok()?,
                to: PortId::try_from("model_ref".to_string()).ok()?,
            },
        ],
        diagnostics: vec![ContractUpgradeDiagnostic {
            reason: ContractUpgradeRejectionReason::UnsupportedLegacyContract,
            message: "llamacpp-inference was migrated to canonical llm-inference; legacy model path evidence must resolve through Pumas before execution.".to_string(),
            node_id: Some(node_id),
            node_type: Some(NodeTypeId::try_from("llamacpp-inference".to_string()).ok()?),
            port_id: None,
        }],
    };
    record.validate().ok()?;
    Some(record)
}

fn legacy_pytorch_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("pytorch-inference".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("pytorch-inference".to_string()).ok()?,
                to: NodeTypeId::try_from("llm-inference".to_string()).ok()?,
            },
            ContractUpgradeChange::PortIdChanged {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                from: PortId::try_from("model_path".to_string()).ok()?,
                to: PortId::try_from("pumas_model_ref".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("temperature".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("max_tokens".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("device".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("model_type".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("environment_ref".to_string()).ok()?,
            },
        ],
        diagnostics: vec![ContractUpgradeDiagnostic {
            reason: ContractUpgradeRejectionReason::UnsupportedLegacyContract,
            message: "pytorch-inference was migrated to canonical llm-inference; legacy model path and model type evidence must resolve through Pumas/Transformers package facts before execution.".to_string(),
            node_id: Some(node_id),
            node_type: Some(NodeTypeId::try_from("pytorch-inference".to_string()).ok()?),
            port_id: None,
        }],
    };
    record.validate().ok()?;
    Some(record)
}

fn legacy_embedding_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("embedding".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("embedding".to_string()).ok()?,
                to: NodeTypeId::try_from("llm-inference".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("model".to_string()).ok()?,
            },
        ],
        diagnostics: vec![ContractUpgradeDiagnostic {
            reason: ContractUpgradeRejectionReason::UnsupportedLegacyContract,
            message: "embedding was migrated to canonical llm-inference with task_kind=embedding; legacy model evidence must resolve through Pumas before execution.".to_string(),
            node_id: Some(node_id),
            node_type: Some(NodeTypeId::try_from("embedding".to_string()).ok()?),
            port_id: None,
        }],
    };
    record.validate().ok()?;
    Some(record)
}

fn legacy_reranker_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("reranker".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("reranker".to_string()).ok()?,
                to: NodeTypeId::try_from("llm-inference".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("model_path".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("top_k".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("return_documents".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                port_id: PortId::try_from("model_path".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                port_id: PortId::try_from("model_ref".to_string()).ok()?,
            },
        ],
        diagnostics: vec![ContractUpgradeDiagnostic {
            reason: ContractUpgradeRejectionReason::UnsupportedLegacyContract,
            message: "reranker was migrated to canonical llm-inference with task_kind=rerank; legacy model path and task options must resolve through Pumas/inference validation before execution.".to_string(),
            node_id: Some(node_id),
            node_type: Some(NodeTypeId::try_from("reranker".to_string()).ok()?),
            port_id: None,
        }],
    };
    record.validate().ok()?;
    Some(record)
}

fn upgrade_record_node_id(record: &ContractUpgradeRecord) -> String {
    record
        .changes
        .iter()
        .find_map(|change| match change {
            ContractUpgradeChange::NodeTypeChanged { node_id, .. }
            | ContractUpgradeChange::PortIdChanged { node_id, .. }
            | ContractUpgradeChange::PortAdded { node_id, .. }
            | ContractUpgradeChange::PortRemoved { node_id, .. } => {
                Some(node_id.as_str().to_string())
            }
            ContractUpgradeChange::VolatileProjectionRegenerated { .. } => None,
        })
        .unwrap_or_default()
}
