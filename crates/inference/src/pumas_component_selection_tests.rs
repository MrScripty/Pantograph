//! Genuine public owner issuance over seeded inert bytes. No assembler, install,
//! wheel import, interpreter execution or actual provider content qualification.
use super::*;
use crate::python_startup_broker::PythonStartupPhase;
use crate::{CapabilityAvailabilityId, RuntimeVariantId};
use pumas_app_manager::VersionManager;
use pumas_library::{
    metadata::{InstalledVersionMetadata, MetadataManager},
    AppId,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::mpsc, time::Duration};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Fixture {
    _root: tempfile::TempDir,
    runtime: PathBuf,
    manager: VersionManager,
}
impl Fixture {
    async fn new() -> Self {
        let source = PathBuf::from(std::env::var_os("PUMAS_COMPONENT_TEST_SOURCE_ROOT").expect(
            "owner fixtures require explicit verified Pumas3d977 source root; no silent skip",
        ));
        let root = tempfile::tempdir().unwrap();
        let depot = root
            .path()
            .join("launcher-data/managed-python/python")
            .join("a".repeat(64));
        let executable = depot.join("bin/python3.12");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"inert interpreter bytes; never executed").unwrap();
        let interpreter_hash = digest(b"inert interpreter bytes; never executed");
        let depot_hash = digest(&serde_json::to_vec(&json!({
            "domain":"pumas.torch.selected-interpreter-manifest.v1",
            "members":[[".pumas-python-depot.lock",0,digest(b"")],
                ["bin/python3.12",std::fs::metadata(&executable).unwrap().len(),interpreter_hash]]
        })).unwrap());
        let dependency = b"inert dependency data; never imported";
        let files =
            json!([{"path":"fixture.data","size":dependency.len(),"sha256":digest(dependency)}]);
        let identity = json!({"domain":"pumas.torch.component-assembly.v1","policy_version":1,
            "qualification":"assembled_unqualified",
            "launch_restrictions":["no_runnable_interpreter_entry","no_initialized_import_provenance"],
            "interpreter_depot_manifest_sha256":depot_hash,
            "component":{"final_dependency_files":files}});
        let encoded = serde_json::to_vec(&identity).unwrap();
        let manifest_hash = digest(&encoded);
        let tag = format!("torch-component-{manifest_hash}");
        let runtime = root
            .path()
            .join(AppId::Torch.versions_dir_name())
            .join(&tag);
        let packages = runtime.join("venv/lib/python3.12/site-packages");
        std::fs::create_dir_all(&packages).unwrap();
        std::fs::write(packages.join("fixture.data"), dependency).unwrap();
        std::fs::write(
            runtime.join("installed-files.json"),
            serde_json::to_vec(&json!({"files":files})).unwrap(),
        )
        .unwrap();
        std::fs::write(runtime.join("component-manifest.json"), encoded).unwrap();
        std::fs::write(runtime.join("runtime.json"), serde_json::to_vec(&json!({
            "python":"python3.12","assembly_only":true,"recipe_id":"pumas-component-assembly-v1",
            "component_revision":tag,"component_manifest_sha256":manifest_hash,
            "managed_python":{"executable":{"path":executable,"sha256":interpreter_hash}}
        })).unwrap()).unwrap();
        // Match the owner's embedded sidecar closure without executing any file.
        for name in [
            "serve.py",
            "validate_runtime.py",
            "resolve_runtime.py",
            "probe_runtime.py",
            "nunchaku_compat.py",
            "control_api.py",
            "device_manager.py",
            "diffusion.py",
            "flux2.py",
            "image_api.py",
            "model_manager.py",
            "owned_worker.py",
            "private_owned_channel.py",
            "owned_audio.py",
            "owned_model_operations.py",
            "speech_operations.py",
            "native_speech_result.py",
            "audio_input.py",
            "audio_contract.py",
            "loaders/cohere_asr_loader.py",
            "loaders/owned_cohere_source.py",
            "speech_binding.py",
            "openai_api.py",
            "validation.py",
            "loaders/__init__.py",
            "loaders/dllm_loader.py",
            "loaders/safetensors_loader.py",
            "loaders/sherry_loader.py",
        ] {
            let destination = runtime.join(name);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::copy(source.join("torch-server").join(name), destination).unwrap();
        }
        let metadata = MetadataManager::new(root.path());
        metadata.ensure_directories().unwrap();
        metadata
            .update_installed_version(
                &tag,
                InstalledVersionMetadata {
                    path: tag.clone(),
                    release_tag: tag.clone(),
                    ..Default::default()
                },
                Some(AppId::Torch),
            )
            .unwrap();
        let manager = VersionManager::new(root.path(), AppId::Torch)
            .await
            .unwrap();
        Self {
            _root: root,
            runtime,
            manager,
        }
    }
    async fn selection(&self) -> TorchComponentSelection {
        self.manager
            .select_torch_component_revision(self.runtime.file_name().unwrap().to_str().unwrap())
            .await
            .unwrap()
    }
}

fn request(selection: &TorchComponentSelection) -> ManagedPythonRuntimeStartRequest {
    ManagedPythonRuntimeStartRequest {
        contract_version: 1,
        environment_id: CapabilityAvailabilityId::parse("consumer.declared.environment").unwrap(),
        runtime_revision: selection.revision_tag().into(),
        runtime_manifest_sha256: selection.manifest_sha256().into(),
        runtime_variant_id: RuntimeVariantId::parse("pytorch.cpu").unwrap(),
        provider_wheel_sha256: crate::managed_python_binding::TOKENIZER_PROVIDER_WHEEL_SHA256
            .into(),
        provider_extension_sha256:
            crate::managed_python_binding::TOKENIZER_PROVIDER_EXTENSION_SHA256.into(),
        provider_build_binding_sha256:
            crate::managed_python_binding::TOKENIZER_PROVIDER_BUILD_BINDING_SHA256.into(),
    }
}

#[tokio::test]
async fn component_actual_owner_source_reservation_rollback_and_pin_keep_exact_arc() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    selection.validate().unwrap();
    assert_eq!(selection.qualification(), "assembled_unqualified");
    assert!(!fixture.runtime.join("venv/bin").exists());
    let source = selection.retained_source().clone();
    let weak = Arc::downgrade(&source);
    let expected = request(&selection);
    let depot = selection.interpreter_depot_manifest_sha256().to_owned();
    let broker = Arc::new(PythonStartupBroker::default());
    let token = broker
        .reserve_component_selection(&expected, &depot, selection)
        .unwrap();
    assert!(Arc::ptr_eq(token.selection().retained_source(), &source));
    assert_eq!(
        token.closed_start_refusal(),
        PumasComponentStartRefusal::AssembledUnqualified
    );
    drop(source);
    drop(token);
    assert!(weak.upgrade().is_none());
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
    let selection = fixture.selection().await;
    let weak = Arc::downgrade(selection.retained_source());
    let token = broker
        .reserve_component_selection(&expected, &depot, selection)
        .unwrap();
    token.pin_for_process().unwrap();
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::ProcessPinned);
    assert!(weak.upgrade().is_some());
    // Only controlled fresh-owner destruction, not real process retirement.
    drop(broker);
    assert!(weak.upgrade().is_none());
}

#[tokio::test]
async fn component_bad_declarations_and_selection_mismatch_precede_validation() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    let baseline = request(&selection);
    let depot = selection.interpreter_depot_manifest_sha256();
    for field in [
        "version",
        "provider",
        "revision",
        "manifest",
        "depot-shape",
        "depot-mismatch",
    ] {
        let mut value = baseline.clone();
        let mut expected_depot = depot.to_owned();
        let expected = match field {
            "version" => {
                value.contract_version = 0;
                PumasComponentStartRefusal::InvalidDeclaration
            }
            "provider" => {
                value.provider_wheel_sha256 = "0".repeat(64);
                PumasComponentStartRefusal::InvalidDeclaration
            }
            "revision" => {
                value.runtime_revision = "different-revision".into();
                PumasComponentStartRefusal::SelectionMismatch
            }
            "manifest" => {
                value.runtime_manifest_sha256 = "0".repeat(64);
                PumasComponentStartRefusal::SelectionMismatch
            }
            "depot-shape" => {
                expected_depot = "F".repeat(64);
                PumasComponentStartRefusal::InvalidDeclaration
            }
            _ => {
                expected_depot = "0".repeat(64);
                PumasComponentStartRefusal::SelectionMismatch
            }
        };
        let broker = Arc::new(PythonStartupBroker::default());
        assert_eq!(
            reserve_with_validation(
                &broker,
                &value,
                &expected_depot,
                selection.clone(),
                |_| panic!("mismatch invoked blocking validation")
            )
            .err(),
            Some(expected)
        );
        assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
    }
}

#[tokio::test]
async fn component_public_owner_matching_refuses_wrong_component_or_depot() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    assert!(fixture
        .manager
        .select_torch_component_revision_matching(
            selection.revision_tag(),
            &"0".repeat(64),
            selection.interpreter_depot_manifest_sha256()
        )
        .await
        .is_err());
    assert!(fixture
        .manager
        .select_torch_component_revision_matching(
            selection.revision_tag(),
            selection.manifest_sha256(),
            &"0".repeat(64)
        )
        .await
        .is_err());
    let matched = fixture
        .manager
        .select_torch_component_revision_matching(
            selection.revision_tag(),
            selection.manifest_sha256(),
            selection.interpreter_depot_manifest_sha256(),
        )
        .await
        .unwrap();
    matched.validate().unwrap();
    assert_eq!(matched.qualification(), "assembled_unqualified");
}

#[tokio::test]
async fn component_changed_selected_bytes_refuse_before_broker_claim() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    let value = request(&selection);
    let depot = selection.interpreter_depot_manifest_sha256().to_owned();
    // Deliberate out-of-protocol mutation challenges validation, not owner deletion safety.
    std::fs::write(
        fixture
            .runtime
            .join("venv/lib/python3.12/site-packages/fixture.data"),
        b"changed selected bytes",
    )
    .unwrap();
    let broker = Arc::new(PythonStartupBroker::default());
    assert_eq!(
        broker
            .reserve_component_selection(&value, &depot, selection)
            .err(),
        Some(PumasComponentStartRefusal::InvalidRetainedBytes)
    );
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
}

#[tokio::test]
async fn component_real_validation_outside_mutex_allows_legacy_winner() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    let value = request(&selection);
    let depot = selection.interpreter_depot_manifest_sha256().to_owned();
    let broker = Arc::new(PythonStartupBroker::default());
    let other = broker.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let validation = std::thread::spawn(move || {
        reserve_with_validation(&other, &value, &depot, selection, |actual| {
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            actual.validate().map_err(|_| ())
        })
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let (won_tx, won_rx) = mpsc::channel();
    let competitor = broker.clone();
    let legacy = std::thread::spawn(move || {
        won_tx.send(competitor.claim_legacy()).unwrap();
    });
    let won = won_rx.recv_timeout(Duration::from_secs(5));
    release_tx.send(()).unwrap();
    let result = validation.join().unwrap();
    legacy.join().unwrap();
    assert_eq!(won.unwrap(), Ok(()));
    assert_eq!(
        result.err(),
        Some(PumasComponentStartRefusal::Reservation(
            PythonStartupReservationRefusal::LegacyClaimed
        ))
    );
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
}

#[tokio::test]
async fn component_two_valid_owner_reservations_admit_exactly_one_after_validation() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    let broker = Arc::new(PythonStartupBroker::default());
    let (ready_tx, ready_rx) = mpsc::channel();
    let mut releases = Vec::new();
    let mut jobs = Vec::new();
    for _ in 0..2 {
        let b = broker.clone();
        let ready = ready_tx.clone();
        let actual = selection.clone();
        let (release_tx, release_rx) = mpsc::channel();
        releases.push(release_tx);
        jobs.push(std::thread::spawn(move || {
            let value = request(&actual);
            let depot = actual.interpreter_depot_manifest_sha256().to_owned();
            reserve_with_validation(&b, &value, &depot, actual, |s| {
                s.validate().map_err(|_| ())?;
                ready.send(()).unwrap();
                release_rx
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|_| ())?;
                Ok(())
            })
        }));
    }
    drop(ready_tx);
    let first = ready_rx.recv_timeout(Duration::from_secs(5));
    let second = ready_rx.recv_timeout(Duration::from_secs(5));
    for release in releases {
        let _ = release.send(());
    }
    let outcomes: Vec<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
    assert!(
        first.is_ok() && second.is_ok(),
        "both real validations must reach bounded admission barrier"
    );
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter_map(|r| r.as_ref().err())
            .copied()
            .collect::<Vec<_>>(),
        vec![PumasComponentStartRefusal::Reservation(
            PythonStartupReservationRefusal::RegistrationBusy
        )]
    );
    drop(outcomes);
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
}

#[tokio::test]
async fn component_declared_environment_is_not_owner_fact_and_never_opens_start() {
    let fixture = Fixture::new().await;
    let selection = fixture.selection().await;
    let depot = selection.interpreter_depot_manifest_sha256();
    for environment in ["consumer.declared.a", "consumer.declared.b"] {
        let mut value = request(&selection);
        value.environment_id = CapabilityAvailabilityId::parse(environment).unwrap();
        let broker = Arc::new(PythonStartupBroker::default());
        let token = broker
            .reserve_component_selection(&value, depot, selection.clone())
            .unwrap();
        assert_eq!(
            token.closed_start_refusal(),
            PumasComponentStartRefusal::AssembledUnqualified
        );
        // The fixture contains no reviewed provider image; these are only request declarations.
        assert_eq!(broker.closed_start_refusal(&value,|| false),crate::managed_python_binding::ManagedPythonBindingRefusal::RegisteredOwnerCustodyUnavailable);
        drop(token);
        assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
    }
}
