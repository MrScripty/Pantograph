use std::sync::Arc;
use std::time::Duration;

use pantograph_dependency_environment_service::{
    resolve_dependency_requirements_payload, DependencyEnvironmentReadinessSnapshot,
    DependencyEnvironmentReadinessSnapshotProvider, DependencyEnvironmentReadinessSnapshotStatus,
    DependencyReadinessWorkQueue, DependencyRequirementsPayload, DependencyRequirementsRegistry,
    DependencyRequirementsRegistryError,
};
use pantograph_dependency_planning::{
    produce_dependency_requirements_proof, DependencyEnvironmentAction,
    DependencyEnvironmentInstallState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentResult, DependencyEnvironmentValidationState,
    ValidatedDependencyEnvironmentRequest, ValidatedDependencyPlanningRequest,
};
use workflow_nodes::setup::PumasSelectorAccess;

use crate::dependency_inventory::DependencyInventoryService;
use crate::EmbeddedRuntimeError;

/// Configuration for the embedded dependency-readiness snapshot producer loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedDependencyReadinessSnapshotProducerConfig {
    pub poll_interval: Duration,
}

impl Default for EmbeddedDependencyReadinessSnapshotProducerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(60),
        }
    }
}

/// Embedded-runtime owner for async dependency-readiness snapshot probes.
#[derive(Clone)]
pub struct EmbeddedDependencyReadinessSnapshotProducer {
    snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
    work_queue: Arc<DependencyReadinessWorkQueue>,
    requirements_registry: Arc<dyn DependencyRequirementsRegistry>,
    dependency_inventory: Arc<DependencyInventoryService>,
    pumas_selector_access: Option<Arc<PumasSelectorAccess>>,
    config: EmbeddedDependencyReadinessSnapshotProducerConfig,
}

impl EmbeddedDependencyReadinessSnapshotProducer {
    #[must_use]
    pub fn new(
        snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
        work_queue: Arc<DependencyReadinessWorkQueue>,
        requirements_registry: Arc<dyn DependencyRequirementsRegistry>,
    ) -> Self {
        Self {
            snapshot_provider,
            work_queue,
            requirements_registry,
            dependency_inventory: Arc::new(DependencyInventoryService::default()),
            pumas_selector_access: None,
            config: EmbeddedDependencyReadinessSnapshotProducerConfig::default(),
        }
    }

    #[must_use]
    pub(crate) fn with_pumas_selector_access(mut self, access: Arc<PumasSelectorAccess>) -> Self {
        self.pumas_selector_access = Some(access);
        self
    }

    #[must_use]
    pub fn with_config(
        mut self,
        config: EmbeddedDependencyReadinessSnapshotProducerConfig,
    ) -> Self {
        self.config = config;
        self
    }

    #[must_use]
    #[cfg(any(test, feature = "standalone"))]
    pub(crate) fn with_dependency_inventory(
        mut self,
        dependency_inventory: Arc<DependencyInventoryService>,
    ) -> Self {
        self.dependency_inventory = dependency_inventory;
        self
    }

    pub fn spawn(
        self,
        runtime_handle: tokio::runtime::Handle,
    ) -> Result<EmbeddedDependencyReadinessSnapshotProducerHandle, EmbeddedRuntimeError> {
        if self.config.poll_interval.is_zero() {
            return Err(EmbeddedRuntimeError::Config {
                message:
                    "dependency-readiness snapshot producer poll interval must be greater than zero"
                        .to_string(),
            });
        }

        let snapshot_provider = self.snapshot_provider;
        let work_queue = self.work_queue;
        let requirements_registry = self.requirements_registry;
        let dependency_inventory = self.dependency_inventory;
        let pumas_selector_access = self.pumas_selector_access;
        let poll_interval = self.config.poll_interval;
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = runtime_handle.spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        while let Some(item) = work_queue.pop_next() {
                            let mut payload_result = resolve_dependency_requirements_payload(
                                requirements_registry.as_ref(),
                                &item.request,
                            );
                            // Only a genuine cold miss permits owner resolution. Stale and
                            // mismatched registry entries retain their existing rejection.
                            if matches!(payload_result, Err(DependencyRequirementsRegistryError::MissingPayload { .. })) {
                                if let Some(access) = pumas_selector_access.as_ref() {
                                    payload_result = resolve_empty_pumas_requirements(access, &item.request).await;
                                    if let Ok(payload) = &payload_result {
                                        if let Err(error) = publish_resolved_requirements(
                                            &snapshot_provider, &item.request, payload,
                                        ) {
                                            log::error!("dependency requirements bootstrap publication failed: {error}");
                                            continue;
                                        }
                                    }
                                }
                            }
                            let snapshot = match payload_result {
                                Ok(payload) => {
                                    dependency_inventory
                                        .snapshot_for_work_item(&item, payload)
                                    .await
                                }
                                Err(error) => {
                                    log::warn!("dependency_bootstrap_diagnostic {}", serde_json::json!({
                                        "phase": "probe_requirements_lookup_rejected",
                                        "request": item.request.as_request(),
                                        "registry_error": format!("{error:?}"),
                                    }));
                                    DependencyEnvironmentReadinessSnapshot::unavailable_for_work_item_registry_error(
                                        &item,
                                        &error,
                                    )
                                }
                            };
                            match snapshot.and_then(|snapshot| snapshot_provider.insert_snapshot(snapshot)) {
                                Ok(()) => {}
                                Err(error) => {
                                    log::error!(
                                        "dependency-readiness snapshot producer failed to publish queued unavailable snapshot: {error}"
                                    );
                                }
                            }
                        }
                        log::trace!(
                            "dependency-readiness snapshot producer heartbeat: {} snapshots available, {} work items queued",
                            snapshot_provider.snapshot_count(),
                            work_queue.len()
                        );
                    }
                }
            }
        });

        Ok(EmbeddedDependencyReadinessSnapshotProducerHandle::new(
            shutdown_tx,
            join_handle,
        ))
    }
}

// This first owner bootstrap supports models whose authoritative Pumas result
// declares no dependencies. Declared profiles need their own projection;
// unresolved/unsupported profiles must never become an empty ready set.
async fn resolve_empty_pumas_requirements(
    access: &PumasSelectorAccess,
    request: &ValidatedDependencyEnvironmentRequest,
) -> Result<DependencyRequirementsPayload, DependencyRequirementsRegistryError> {
    let raw = request.as_request();
    let planning = &raw.planning_request;
    let unsupported = || DependencyRequirementsRegistryError::InvalidPayload {
        field: "dependency_requirements_payload.pumas_resolution",
        reason:
            "Pumas resolution is unavailable, mismatched, or needs declared dependency projection",
    };
    let proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(planning.clone())
            .map_err(DependencyRequirementsRegistryError::InvalidContract)?,
        None,
    )
    .map_err(DependencyRequirementsRegistryError::InvalidContract)?;
    if raw.action != DependencyEnvironmentAction::Check
        || raw.dependency_requirements_id.as_ref() != Some(&proof.dependency_requirements_id)
        || !planning.selected_binding_ids.is_empty()
        || !planning.dependency_override_patches.is_empty()
        || !planning.trait_intents.is_empty()
    {
        return Err(unsupported());
    }
    let platform = planning.platform_context.as_ref().ok_or_else(unsupported)?;
    if platform.platform_key.as_str()
        != format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    {
        return Err(unsupported());
    }
    let backend = planning
        .scheduler_intent
        .requested_runtime_id
        .as_ref()
        .ok_or_else(unsupported)?;
    // Reuse the selected package projection's model/revision/artifact guards.
    let facts = crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsSource::new(Some(
        Arc::new(access.clone()),
    ))
    .collect(&planning.model_ref)
    .await;
    let crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsBridgeOutcome::Projected {
        facts,
        ..
    } = facts
    else {
        return Err(unsupported());
    };
    if facts.validation_state != inference::ModelValidationState::Valid {
        return Err(unsupported());
    }
    let resolution = access
        .resolve_model_dependency_requirements(
            &planning.model_ref.model_id,
            platform.platform_key.as_str(),
            Some(backend.as_str()),
        )
        .await
        .map_err(|error| {
            log::warn!("Pumas dependency requirements resolution failed: {error}");
            unsupported()
        })?;
    if resolution.model_id
        != planning
            .model_ref
            .model_id
            .strip_prefix("pumas://models/")
            .unwrap_or(&planning.model_ref.model_id)
        || resolution.platform_key != platform.platform_key.as_str()
        || resolution.backend_key.as_deref() != Some(backend.as_str())
        || resolution.dependency_contract_version
            != pumas_library::model_library::DEPENDENCY_CONTRACT_VERSION
        || resolution.validation_state
            != pumas_library::model_library::DependencyValidationState::Resolved
        || !resolution.validation_errors.is_empty()
        || !resolution.bindings.is_empty()
    {
        log::warn!(
            "Pumas dependency requirements bootstrap rejected: {}",
            serde_json::json!(resolution)
        );
        return Err(unsupported());
    }
    log::info!(
        "dependency_bootstrap_diagnostic {}",
        serde_json::json!({
            "phase": "pumas_requirements_resolved", "request": raw, "resolution": resolution,
        })
    );
    DependencyRequirementsPayload::new(
        proof.dependency_requirements_id,
        raw.identity_key.clone(),
        vec![],
        vec![],
        vec![],
    )
}

fn publish_resolved_requirements(
    provider: &DependencyEnvironmentReadinessSnapshotProvider,
    request: &ValidatedDependencyEnvironmentRequest,
    payload: &DependencyRequirementsPayload,
) -> Result<(), pantograph_dependency_environment_service::DependencyEnvironmentSnapshotStoreError>
{
    let mut resolve_request = request.as_request().clone();
    resolve_request.action = DependencyEnvironmentAction::Resolve;
    let resolve_request = ValidatedDependencyEnvironmentRequest::try_from(resolve_request)
        .map_err(pantograph_dependency_environment_service::DependencyEnvironmentSnapshotStoreError::InvalidSnapshotKey)?;
    let result = DependencyEnvironmentResult {
        contract_version: 1,
        action: DependencyEnvironmentAction::Resolve,
        identity_key: payload.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::Resolved,
        install_state: DependencyEnvironmentInstallState::NotRequested,
        validation_state: DependencyEnvironmentValidationState::Valid,
        failure_state: None,
        dependency_requirements_id: Some(payload.dependency_requirements_id.clone()),
        environment_ref: None,
        requirements: payload.requirements.clone(),
        bindings: payload.bindings.clone(),
        selected_binding_ids: payload.selected_binding_ids.clone(),
        binding_statuses: vec![],
        operation: None,
        validation_errors: vec![],
        diagnostics: vec![],
    };
    provider.insert_snapshot(DependencyEnvironmentReadinessSnapshot::for_request(
        &resolve_request,
        result,
        DependencyEnvironmentReadinessSnapshotStatus::Fresh,
    )?)
}

/// Handle for a tracked dependency-readiness snapshot producer task.
#[derive(Debug)]
pub struct EmbeddedDependencyReadinessSnapshotProducerHandle {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl EmbeddedDependencyReadinessSnapshotProducerHandle {
    fn new(
        shutdown_tx: tokio::sync::watch::Sender<bool>,
        join_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            shutdown_tx,
            join_handle: tokio::sync::Mutex::new(Some(join_handle)),
        }
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(join_handle) = self.join_handle.lock().await.take() {
            match join_handle.await {
                Ok(()) => {}
                Err(error) if error.is_panic() => {
                    log::error!(
                        "dependency-readiness snapshot producer panicked during shutdown: {error}"
                    );
                }
                Err(error) => {
                    log::warn!(
                        "dependency-readiness snapshot producer was cancelled during shutdown: {error}"
                    );
                }
            }
        }
    }
}
