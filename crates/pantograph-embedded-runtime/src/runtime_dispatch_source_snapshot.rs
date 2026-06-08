use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use pantograph_dependency_planning::PumasModelRef;
use pantograph_scheduler::SchedulerTaskStateRecord;
use pantograph_workflow_service::workflow::{
    WorkflowRuntimeDispatchSourceRefreshError, WorkflowRuntimeDispatchSourceRefresher,
    WorkflowSchedulerTask,
};

use crate::pumas_dispatch_package_facts::{
    PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsSource,
};
use crate::runtime_dispatch_capability_facts::{
    RuntimeDispatchCapabilityFactsOutcome, RuntimeDispatchCapabilityFactsSource,
};

pub(crate) const EMBEDDED_RUNTIME_DISPATCH_SOURCE_FACT_SNAPSHOT_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmbeddedRuntimeDispatchCandidateSourceSnapshot {
    pub(crate) contract_version: u16,
    pub(crate) snapshot_version: u64,
    pub(crate) refreshed_at_ms: u64,
    pub(crate) model_ref: Option<PumasModelRef>,
    pub(crate) pumas_package_facts: Option<PumasDispatchPackageFactsBridgeOutcome>,
    pub(crate) runtime_capability_facts: Option<RuntimeDispatchCapabilityFactsOutcome>,
    pub(crate) diagnostics: Vec<EmbeddedRuntimeDispatchSourceSnapshotDiagnostic>,
}

impl Default for EmbeddedRuntimeDispatchCandidateSourceSnapshot {
    fn default() -> Self {
        Self {
            contract_version: EMBEDDED_RUNTIME_DISPATCH_SOURCE_FACT_SNAPSHOT_CONTRACT_VERSION,
            snapshot_version: 0,
            refreshed_at_ms: 0,
            model_ref: None,
            pumas_package_facts: None,
            runtime_capability_facts: None,
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedRuntimeDispatchSourceSnapshotDiagnostic {
    pub(crate) code: EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode {
    MissingSnapshot,
    StaleSnapshot,
    ModelRefMismatch,
    InvalidContractVersion,
    PathCarryingModelRef,
}

#[derive(Clone)]
pub(crate) struct EmbeddedRuntimeDispatchSourceFactSnapshotStore {
    pumas_source: PumasDispatchPackageFactsSource,
    runtime_capability_source: RuntimeDispatchCapabilityFactsSource,
    max_snapshot_age_ms: u64,
    state: Arc<Mutex<EmbeddedRuntimeDispatchSourceFactSnapshotState>>,
}

#[derive(Debug, Default)]
struct EmbeddedRuntimeDispatchSourceFactSnapshotState {
    next_snapshot_version: u64,
    current_snapshot: Option<EmbeddedRuntimeDispatchCandidateSourceSnapshot>,
}

#[derive(Clone)]
pub(crate) struct EmbeddedRuntimeDispatchSourceFactRefresher {
    snapshot_store: EmbeddedRuntimeDispatchSourceFactSnapshotStore,
}

impl EmbeddedRuntimeDispatchSourceFactSnapshotStore {
    pub(crate) fn new(
        pumas_source: PumasDispatchPackageFactsSource,
        runtime_capability_source: RuntimeDispatchCapabilityFactsSource,
        max_snapshot_age_ms: u64,
    ) -> Self {
        Self {
            pumas_source,
            runtime_capability_source,
            max_snapshot_age_ms,
            state: Arc::new(Mutex::new(EmbeddedRuntimeDispatchSourceFactSnapshotState {
                next_snapshot_version: 1,
                current_snapshot: None,
            })),
        }
    }

    pub(crate) async fn refresh_for_model_ref(
        &self,
        model_ref: &PumasModelRef,
        refreshed_at_ms: u64,
    ) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
        let diagnostics = validate_model_ref(model_ref);
        let (pumas_package_facts, runtime_capability_facts) = if diagnostics.is_empty() {
            (
                Some(self.pumas_source.collect(model_ref).await),
                Some(self.runtime_capability_source.collect()),
            )
        } else {
            (None, None)
        };

        let mut state = self
            .state
            .lock()
            .expect("dispatch source snapshot state should not be poisoned");
        let snapshot = EmbeddedRuntimeDispatchCandidateSourceSnapshot {
            contract_version: EMBEDDED_RUNTIME_DISPATCH_SOURCE_FACT_SNAPSHOT_CONTRACT_VERSION,
            snapshot_version: state.next_snapshot_version,
            refreshed_at_ms,
            model_ref: Some(model_ref.clone()),
            pumas_package_facts,
            runtime_capability_facts,
            diagnostics,
        };
        state.next_snapshot_version = state
            .next_snapshot_version
            .checked_add(1)
            .expect("dispatch source snapshot versions should not overflow");
        state.current_snapshot = Some(snapshot.clone());
        snapshot
    }

    pub(crate) fn snapshot_for_dispatch(
        &self,
        model_ref: &PumasModelRef,
        now_ms: u64,
    ) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
        let state = self
            .state
            .lock()
            .expect("dispatch source snapshot state should not be poisoned");
        let Some(snapshot) = state.current_snapshot.clone() else {
            return unavailable_snapshot(diagnostic(
                EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::MissingSnapshot,
                "runtime dispatch source-fact snapshot has not been refreshed",
            ));
        };
        validate_snapshot_for_dispatch(snapshot, model_ref, now_ms, self.max_snapshot_age_ms)
    }
}

impl EmbeddedRuntimeDispatchSourceFactRefresher {
    #[must_use]
    pub(crate) fn new(snapshot_store: EmbeddedRuntimeDispatchSourceFactSnapshotStore) -> Self {
        Self { snapshot_store }
    }
}

#[async_trait]
impl WorkflowRuntimeDispatchSourceRefresher for EmbeddedRuntimeDispatchSourceFactRefresher {
    async fn refresh_runtime_dispatch_sources(
        &self,
        _task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &pantograph_dependency_planning::DependencyReadinessProofEnvelope,
    ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
        self.snapshot_store
            .refresh_for_model_ref(
                &readiness_proof.preflight_result.identity_key.model_ref,
                current_time_ms(),
            )
            .await;
        Ok(())
    }
}

fn validate_snapshot_for_dispatch(
    mut snapshot: EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    model_ref: &PumasModelRef,
    now_ms: u64,
    max_snapshot_age_ms: u64,
) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
    let mut diagnostics = Vec::new();
    if snapshot.contract_version != EMBEDDED_RUNTIME_DISPATCH_SOURCE_FACT_SNAPSHOT_CONTRACT_VERSION
    {
        diagnostics.push(diagnostic(
            EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::InvalidContractVersion,
            "runtime dispatch source-fact snapshot contract version is unsupported",
        ));
    }
    if snapshot
        .model_ref
        .as_ref()
        .is_none_or(|snapshot_model_ref| snapshot_model_ref != model_ref)
    {
        diagnostics.push(diagnostic(
            EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::ModelRefMismatch,
            "runtime dispatch source-fact snapshot model ref does not match the dispatch model ref",
        ));
    }
    if now_ms.saturating_sub(snapshot.refreshed_at_ms) > max_snapshot_age_ms {
        diagnostics.push(diagnostic(
            EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::StaleSnapshot,
            "runtime dispatch source-fact snapshot is stale",
        ));
    }
    diagnostics.extend(validate_model_ref(model_ref));

    if diagnostics.is_empty() {
        return snapshot;
    }

    snapshot.pumas_package_facts = None;
    snapshot.runtime_capability_facts = None;
    snapshot.diagnostics.extend(diagnostics);
    snapshot
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn validate_model_ref(
    model_ref: &PumasModelRef,
) -> Vec<EmbeddedRuntimeDispatchSourceSnapshotDiagnostic> {
    if model_ref.selected_artifact_path.is_none() {
        return Vec::new();
    }
    vec![diagnostic(
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::PathCarryingModelRef,
        "runtime dispatch source-fact snapshot rejected a path-carrying Pumas model ref",
    )]
}

fn unavailable_snapshot(
    diagnostic: EmbeddedRuntimeDispatchSourceSnapshotDiagnostic,
) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
    EmbeddedRuntimeDispatchCandidateSourceSnapshot {
        diagnostics: vec![diagnostic],
        ..EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
    }
}

fn diagnostic(
    code: EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode,
    message: &str,
) -> EmbeddedRuntimeDispatchSourceSnapshotDiagnostic {
    EmbeddedRuntimeDispatchSourceSnapshotDiagnostic {
        code,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pantograph_runtime_registry::{
        RuntimeDispatchIdentity, RuntimeRegistration, RuntimeRegistry, RuntimeTransition,
    };

    use super::*;

    #[tokio::test]
    async fn store_refreshes_versioned_snapshot_from_source_owners() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["diffusers".to_string()])
                .with_dispatch_identity(dispatch_identity()),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime.pytorch.001".to_string()),
                },
            )
            .expect("runtime should transition to ready");
        let store = EmbeddedRuntimeDispatchSourceFactSnapshotStore::new(
            PumasDispatchPackageFactsSource::new(None),
            RuntimeDispatchCapabilityFactsSource::new(registry),
            100,
        );

        let refreshed = store
            .refresh_for_model_ref(&model_ref("pumas.model.sdxl"), 1_000)
            .await;
        let snapshot = store.snapshot_for_dispatch(&model_ref("pumas.model.sdxl"), 1_050);

        assert_eq!(refreshed.snapshot_version, 1);
        assert_eq!(snapshot.snapshot_version, 1);
        assert!(snapshot.diagnostics.is_empty());
        assert!(matches!(
            snapshot.pumas_package_facts,
            Some(PumasDispatchPackageFactsBridgeOutcome::Unavailable { .. })
        ));
        assert!(matches!(
            snapshot.runtime_capability_facts,
            Some(RuntimeDispatchCapabilityFactsOutcome::Projected { .. })
        ));
    }

    fn dispatch_identity() -> RuntimeDispatchIdentity {
        RuntimeDispatchIdentity::new("diffusers", "runtime.diffusers.pytorch.shared")
            .expect("dispatch identity fixture")
    }

    #[tokio::test]
    async fn store_rejects_stale_snapshot_without_returning_source_facts() {
        let store = EmbeddedRuntimeDispatchSourceFactSnapshotStore::new(
            PumasDispatchPackageFactsSource::new(None),
            RuntimeDispatchCapabilityFactsSource::new(Arc::new(RuntimeRegistry::new())),
            100,
        );
        store
            .refresh_for_model_ref(&model_ref("pumas.model.sdxl"), 1_000)
            .await;

        let snapshot = store.snapshot_for_dispatch(&model_ref("pumas.model.sdxl"), 1_500);

        assert!(snapshot.pumas_package_facts.is_none());
        assert!(snapshot.runtime_capability_facts.is_none());
        assert!(snapshot.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::StaleSnapshot
        }));
    }

    #[tokio::test]
    async fn store_rejects_model_ref_mismatch_without_returning_source_facts() {
        let store = EmbeddedRuntimeDispatchSourceFactSnapshotStore::new(
            PumasDispatchPackageFactsSource::new(None),
            RuntimeDispatchCapabilityFactsSource::new(Arc::new(RuntimeRegistry::new())),
            100,
        );
        store
            .refresh_for_model_ref(&model_ref("pumas.model.sdxl"), 1_000)
            .await;

        let snapshot = store.snapshot_for_dispatch(&model_ref("pumas.model.other"), 1_050);

        assert!(snapshot.pumas_package_facts.is_none());
        assert!(snapshot.runtime_capability_facts.is_none());
        assert!(snapshot.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::ModelRefMismatch
        }));
    }

    #[test]
    fn store_reports_missing_snapshot() {
        let store = EmbeddedRuntimeDispatchSourceFactSnapshotStore::new(
            PumasDispatchPackageFactsSource::new(None),
            RuntimeDispatchCapabilityFactsSource::new(Arc::new(RuntimeRegistry::new())),
            100,
        );

        let snapshot = store.snapshot_for_dispatch(&model_ref("pumas.model.sdxl"), 1_000);

        assert_eq!(snapshot.snapshot_version, 0);
        assert!(snapshot.pumas_package_facts.is_none());
        assert!(snapshot.runtime_capability_facts.is_none());
        assert!(snapshot.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::MissingSnapshot
        }));
    }

    fn model_ref(model_id: &str) -> PumasModelRef {
        PumasModelRef {
            model_id: model_id.to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }
    }
}
