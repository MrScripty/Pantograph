use super::*;
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyReadinessWorkItem, DependencyReadinessWorkItemProvenance,
};
use pantograph_workflow_service::{
    graph::{authored_snapshot_from_descriptor, WorkflowGraph},
    WorkflowDependencyReadinessComponents, WorkflowGraphCurrentValidationRefreshRequest,
    WorkflowGraphEditSessionCreateRequest,
};
use workflow_nodes::setup::PumasSelectorAccess;

use crate::dependency_inventory::{DependencyInventoryProvider, DependencyInventoryRequest};
use crate::dependency_inventory_python::PythonPackageDependencyInventoryProvider;
use crate::inference_interface_facts_provider::EmbeddedInferenceInterfaceFactsProvider;
use crate::package_readiness_provider::{
    PackageReadinessProbeOutcome, PackageReadinessProbeRequest, PackageReadinessProbeRunner,
};

const MODEL: &str = "llm/qualification/synthetic-gpt2";

fn request() -> DependencyEnvironmentRequest {
    let planning: DependencyPlanningRequest = serde_json::from_value(serde_json::json!({
        "model_ref":{"model_id":MODEL,"selected_artifact_id":"main"},
        "task_id":"text_generation",
        "scheduler_intent":{"requested_runtime_id":"pytorch","requested_device_id":"cpu"},
        "platform_context": DependencyPlanningPlatformContext::from_os_arch(std::env::consts::OS,std::env::consts::ARCH).unwrap(),
    })).unwrap();
    let proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(planning.clone()).unwrap(),
        None,
    )
    .unwrap();
    DependencyEnvironmentRequest {
        contract_version: 1,
        action: DependencyEnvironmentAction::Resolve,
        identity_key: proof.identity_key,
        planning_request: planning,
        dependency_requirements_id: Some(proof.dependency_requirements_id),
        environment_ref: None,
    }
}

fn payload(request: &DependencyEnvironmentRequest) -> DependencyRequirementsPayload {
    DependencyRequirementsPayload::new(
        request.dependency_requirements_id.clone().unwrap(),
        request.identity_key.clone(),
        requirements(),
        bindings(),
        binding_names()
            .iter()
            .map(|id| id.parse().unwrap())
            .collect(),
    )
    .unwrap()
}

fn inventory_request(
    mut request: DependencyEnvironmentRequest,
    payload: DependencyRequirementsPayload,
) -> DependencyInventoryRequest {
    request.action = DependencyEnvironmentAction::Check;
    let item = DependencyReadinessWorkItem::new(
        DependencyReadinessWorkItemProvenance::new(
            "text-session".parse().unwrap(),
            "text-run".parse().unwrap(),
            "text-task".parse().unwrap(),
        ),
        ValidatedDependencyEnvironmentRequest::try_from(request).unwrap(),
    );
    DependencyInventoryRequest::new(&item, payload)
}

struct NeverProcessProbe;
#[async_trait]
impl PackageReadinessProbeRunner for NeverProcessProbe {
    async fn probe(&self, _: PackageReadinessProbeRequest) -> PackageReadinessProbeOutcome {
        panic!("embedded text profile must not consult another Python executable")
    }
}

#[tokio::test]
async fn exact_text_inventory_uses_linked_interpreter_and_rejects_altered_profiles() {
    let request = request();
    let payload = payload(&request);
    let provider = PythonPackageDependencyInventoryProvider::new(Arc::new(NeverProcessProbe));
    let observed = provider
        .observe(inventory_request(request.clone(), payload.clone()))
        .await;
    let installed = inference::backend::pytorch::observe_text_package_presence().unwrap();
    assert!(observed.diagnostics.is_empty());
    assert_eq!(observed.rows.len(), TEXT_GENERATION_PACKAGES.len());
    for (row, name) in observed.rows.iter().zip(TEXT_GENERATION_PACKAGES) {
        let expected = if installed.iter().any(|value| value == name) {
            DependencyInventoryObservationState::Ready
        } else {
            DependencyInventoryObservationState::Missing
        };
        assert_eq!(row.state, expected);
    }
    let mut altered = payload.clone();
    altered.requirements[0].version_constraint = Some("==0.0.0".into());
    let observed = provider
        .observe(inventory_request(request.clone(), altered))
        .await;
    assert!(observed
        .rows
        .iter()
        .all(|row| row.state == DependencyInventoryObservationState::Invalid));
    assert!(observed.diagnostics.iter().all(|diagnostic| diagnostic
        .field_path
        .as_ref()
        .is_some_and(|field| field.as_str() == "dependency_environment.bindings")));
    let mut foreign = request;
    foreign.environment_ref = Some(DependencyEnvironmentRef {
        environment_id: "python:foreign".parse().unwrap(),
        manifest_id: None,
    });
    let observed = provider.observe(inventory_request(foreign, payload)).await;
    assert!(observed
        .rows
        .iter()
        .all(|row| row.state == DependencyInventoryObservationState::Invalid));
}

#[test]
fn profile_requires_exact_host_task_runtime_and_complete_binding_choices() {
    let valid = request();
    assert!(supported_request(&valid));
    assert!(is_text_payload(&valid, &payload(&valid)));
    for change in ["device", "task", "runtime", "path", "bindings", "platform"] {
        let mut invalid = valid.clone();
        match change {
            "device" => {
                invalid.identity_key.scheduler_intent.requested_device_id =
                    Some("cuda:0".parse().unwrap())
            }
            "task" => invalid.identity_key.task_id = "chat_completion".parse().unwrap(),
            "runtime" => {
                invalid.identity_key.scheduler_intent.requested_runtime_id =
                    Some("candle".parse().unwrap())
            }
            "path" => {
                invalid.identity_key.model_ref.selected_artifact_path = Some("/tmp/model".into())
            }
            "bindings" => {
                invalid.identity_key.selected_binding_ids =
                    vec![binding_names()[0].parse().unwrap()]
            }
            "platform" => invalid.identity_key.platform_context = None,
            _ => unreachable!(),
        }
        assert!(!supported_request(&invalid), "{change}");
    }
}

async fn owner(
    extra: serde_json::Value,
) -> (
    tempfile::TempDir,
    Arc<pumas_library::PumasApi>,
    PumasDispatchPackageFactsSource,
    RuntimeDispatchCapabilityFactsSource,
) {
    let root = tempfile::tempdir().unwrap();
    for directory in [
        "launcher-data/metadata",
        "launcher-data/cache",
        "launcher-data/logs",
    ] {
        std::fs::create_dir_all(root.path().join(directory)).unwrap();
    }
    let model = root.path().join("shared-resources/models").join(MODEL);
    std::fs::create_dir_all(&model).unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../inference/tests/fixtures/tiny_chat_gpt2/model-11");
    for entry in std::fs::read_dir(fixture).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), model.join(entry.file_name())).unwrap();
        }
    }
    let mut metadata = serde_json::json!({
        "schema_version":2,"model_id":MODEL,"family":"qualification","model_type":"llm",
        "official_name":"Synthetic GPT2","cleaned_name":"synthetic-gpt2","source_path":model,"entry_path":model,
        "storage_kind":"library_owned","selected_artifact_id":"main","selected_artifact_files":["model.safetensors"],
        "import_state":"ready","validation_state":"valid","pipeline_tag":"text-generation","task_type_primary":"text_generation",
        "input_modalities":["text"],"output_modalities":["text"],"recommended_backend":"transformers","runtime_engine_hints":["transformers"]
    });
    metadata
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    std::fs::write(
        model.join("metadata.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let api = Arc::new(
        crate::pumas_test_support::builder(root.path())
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .unwrap(),
    );
    api.rebuild_model_index().await.unwrap();
    let packages = PumasDispatchPackageFactsSource::new(Some(Arc::new(
        PumasSelectorAccess::Owner(api.clone()),
    )));
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::pytorch::PyTorchBackend::new()),
        "PyTorch",
    ));
    let capabilities = RuntimeDispatchCapabilityFactsSource::new(Arc::new(
        pantograph_runtime_registry::RuntimeRegistry::new(),
    ))
    .with_gateway(gateway);
    (root, api, packages, capabilities)
}

#[tokio::test]
async fn pumas_owner_resolves_text_prerequisites_without_asserting_readiness() {
    let (_root, _api, packages, capabilities) = owner(serde_json::json!({})).await;
    let validated = ValidatedDependencyEnvironmentRequest::try_from(request()).unwrap();
    let package = packages
        .collect(&validated.as_request().identity_key.model_ref)
        .await;
    assert!(
        matches!(
            package,
            PumasDispatchPackageFactsBridgeOutcome::Projected { .. }
        ),
        "{package:?}"
    );
    let runtime = capabilities.collect().await;
    assert!(
        matches!(
            runtime,
            RuntimeDispatchCapabilityFactsOutcome::Projected { .. }
        ),
        "{runtime:?}"
    );
    let resolved = resolve_text_requirements(&packages, &capabilities, &validated)
        .await
        .unwrap()
        .expect("text prerequisite resolution");
    let result = resolved.as_result();
    assert_eq!(result.identity_key, validated.as_request().identity_key);
    assert_eq!(
        result.dependency_requirements_id,
        validated.as_request().dependency_requirements_id
    );
    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Resolved
    );
    assert!(result.binding_statuses.is_empty());
    assert!(is_text_payload(
        validated.as_request(),
        &DependencyRequirementsPayload::from_result(&resolved).unwrap()
    ));
    let mut mismatched = validated.into_inner();
    mismatched.dependency_requirements_id = Some("wrong-proof".parse().unwrap());
    assert!(resolve_text_requirements(
        &packages,
        &capabilities,
        &ValidatedDependencyEnvironmentRequest::try_from(mismatched).unwrap()
    )
    .await
    .unwrap()
    .is_none());
}

#[tokio::test]
async fn pumas_declared_dependencies_are_not_replaced_by_presence_prerequisites() {
    let (_root, api, packages, capabilities) = owner(serde_json::json!({})).await;
    api.model_library()
        .index()
        .upsert_dependency_profile(&pumas_library::index::DependencyProfileRecord {
            profile_id: "unknown-pinned-profile".into(),
            profile_version: 1,
            profile_hash: None,
            environment_kind: "python-venv".into(),
            spec_json:
                serde_json::json!({"python_packages":[{"name":"torch","version":"==0.0.0"}]})
                    .to_string(),
            created_at: "2026-10-13T00:00:00Z".into(),
        })
        .unwrap();
    api.model_library()
        .index()
        .upsert_model_dependency_binding(&pumas_library::index::ModelDependencyBindingRecord {
            binding_id: "model-pinned-python".into(),
            model_id: MODEL.into(),
            profile_id: "unknown-pinned-profile".into(),
            profile_version: 1,
            binding_kind: "required_core".into(),
            backend_key: Some("pytorch".into()),
            platform_selector: None,
            status: "active".into(),
            priority: 100,
            attached_by: Some("test".into()),
            attached_at: "2026-10-13T00:00:00Z".into(),
            profile_hash: None,
            environment_kind: None,
            spec_json: None,
        })
        .unwrap();
    let request = ValidatedDependencyEnvironmentRequest::try_from(request()).unwrap();
    assert!(!packages
        .has_no_declared_dependencies(
            MODEL,
            request
                .as_request()
                .identity_key
                .platform_context
                .as_ref()
                .unwrap()
                .platform_key
                .as_str(),
            "pytorch"
        )
        .await
        .unwrap());
    assert!(
        resolve_text_requirements(&packages, &capabilities, &request)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn public_graph_resolve_hands_text_payload_to_readiness_registry() {
    use pantograph_inference_interface_contracts::{
        DependencyEnvironmentActionIntent, DependencyEnvironmentActionIntentStatus,
    };
    let (_root, _api, packages, capabilities) = owner(serde_json::json!({})).await;
    let components = WorkflowDependencyReadinessComponents::new();
    let service = components
        .workflow_service()
        .with_graph_session_fact_providers(
            Arc::new(EmbeddedInferenceInterfaceFactsProvider::new(
                packages,
                capabilities,
            )),
            components.snapshot_provider(),
        );
    let mut graph: WorkflowGraph=serde_json::from_value(serde_json::json!({
        "nodes":[
            {"id":"infer","node_type":"llm-inference","position":{"x":0,"y":0},"data":{"task_kind":"text_generation","runtime":"pytorch","device":"cpu","runtime_source_context":{"operation_type":"text_generation.prompt","context_shape_key":"text.single-prompt","cancellation_mode":"run_scoped"},"pumas_model_ref":{"model_id":MODEL,"selected_artifact_id":"main"}}},
            {"id":"deps","node_type":"dependency-environment","position":{"x":0,"y":100},"data":{"mode":"manual"}}
        ],"edges":[{"id":"dependency","source":"deps","source_handle":"dependency_environment_sidecar","target":"infer","target_handle":"dependency_environment_sidecar"}]
    })).unwrap();
    assert_eq!(graph.nodes[0].id, "infer");
    let preview = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
            workflow_id: None,
        })
        .await
        .unwrap();
    let current = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: preview.session_id,
                graph_revision: preview.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap();
    assert_eq!(current.node_projections.len(), 1, "{current:?}");
    graph.nodes[0].data["inference_interface_snapshot"] = serde_json::to_value(
        authored_snapshot_from_descriptor(&current.node_projections[0].descriptor).unwrap(),
    )
    .unwrap();
    let session = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: None,
        })
        .await
        .unwrap();
    let current = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: session.session_id.clone(),
                graph_revision: session.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap();
    let result = service
        .workflow_graph_resolve_dependency_environment_action_intent(
            DependencyEnvironmentActionIntent {
                contract_version: 1,
                graph_session_id: session.session_id.parse().unwrap(),
                graph_revision: session.graph_revision.parse().unwrap(),
                validation_session_id: current.summary.validation_session_id,
                target_node_id: "deps".parse().unwrap(),
                action: DependencyEnvironmentAction::Resolve,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        result.status,
        DependencyEnvironmentActionIntentStatus::RequestReady,
        "{result:?}"
    );
    assert_eq!(
        components.requirements_registry().len(),
        1,
        "public graph action publishes the payload"
    );
    assert_eq!(
        components.snapshot_provider().snapshot_count(),
        0,
        "Resolve does not forge inventory Ready"
    );
}

#[tokio::test]
async fn text_inventory_snapshot_preserves_embedded_environment_with_default_and_explicit_checks() {
    let mut request = request();
    let payload = payload(&request);
    let inventory =
        crate::dependency_inventory::DependencyInventoryService::from_package_probe_runner(
            Arc::new(NeverProcessProbe),
        );
    for explicit_environment in [false, true] {
        request.environment_ref = explicit_environment.then(environment_ref);
        let context = inventory_request(request.clone(), payload.clone());
        let snapshot = inventory
            .snapshot_for_work_item(&context.item, context.payload)
            .await
            .unwrap();
        assert_eq!(snapshot.result.environment_ref, Some(environment_ref()));
        assert!(snapshot
            .result
            .binding_statuses
            .iter()
            .all(|status| matches!(
                status.state,
                DependencyBindingStatusState::Ready | DependencyBindingStatusState::Missing
            )));
    }
}

#[tokio::test]
async fn text_resolver_refuses_model_custom_code_and_unowned_runtime() {
    let (root, _api, packages, capabilities) = owner(serde_json::json!({})).await;
    let validated = ValidatedDependencyEnvironmentRequest::try_from(request()).unwrap();
    let no_runtime = RuntimeDispatchCapabilityFactsSource::new(Arc::new(
        pantograph_runtime_registry::RuntimeRegistry::new(),
    ));
    assert!(
        resolve_text_requirements(&packages, &no_runtime, &validated)
            .await
            .unwrap()
            .is_none()
    );
    let config_path = root
        .path()
        .join("shared-resources/models")
        .join(MODEL)
        .join("config.json");
    let mut config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
    config["auto_map"] = serde_json::json!({"AutoModelForCausalLM":"custom.RemoteModel"});
    std::fs::write(config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let PumasDispatchPackageFactsBridgeOutcome::Projected { facts, .. } = packages
        .collect(&validated.as_request().identity_key.model_ref)
        .await
    else {
        panic!("current owner package facts");
    };
    assert!(
        facts.requires_custom_code,
        "negative fixture must reach custom-code guard"
    );
    assert!(
        resolve_text_requirements(&packages, &capabilities, &validated)
            .await
            .unwrap()
            .is_none()
    );
}
