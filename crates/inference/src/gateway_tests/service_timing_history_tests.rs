//! Controlled identity-protocol tests, never native calibration or production provenance.
use super::*;
use crate::RuntimeServiceTimingOwnerAttestation;
use pantograph_timing_contracts::RuntimeServiceTimingOwnerProvenance;

fn owner() -> RuntimeServiceTimingOwnerAttestation {
    RuntimeServiceTimingOwnerAttestation {
        owner_fence: "controlled-native-owner:0".into(),
        content_fingerprint: "1".repeat(64),
        facts: crate::RuntimeServiceTimingOwnerFacts {
            implementation_fingerprint: "2".repeat(64),
            effective_configuration_fingerprint: "3".repeat(64),
            physical_device_fingerprint: "4".repeat(64),
            device_id: "cpu".parse().unwrap(),
        },
    }
}
fn protocol_clone(row: &RuntimeServiceTimingAttempt) -> RuntimeServiceTimingAttempt {
    let mut row = row.clone();
    assert_eq!(
        row.capture.as_ref().unwrap().owner_provenance,
        RuntimeServiceTimingOwnerProvenance::Injected
    );
    row.capture.as_mut().unwrap().owner_provenance = RuntimeServiceTimingOwnerProvenance::BuiltIn;
    row
}

#[tokio::test]
async fn history_reloads_preserve_raw_instance_and_fences_but_share_comparable_key() {
    let (_directory, mut request, mut target, decision) = crate::selected_text_execution::fixture();
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let mut backend = Backend::new(clock.clone());
    backend.native_owner = Some(owner());
    let gateway = instrument(backend, recorder.clone(), clock);
    execute(&gateway, request.clone(), target.clone(), decision.clone())
        .await
        .unwrap();
    request.request_id = Some("other-correlation-id".into());
    target.content_fingerprint = Some("unverified-package-label".into());
    execute(&gateway, request, target, decision).await.unwrap();
    let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
    let rows = recorder.rows.lock().unwrap();
    let first = rows[0].history.as_ref().unwrap();
    let second = rows[1].history.as_ref().unwrap();
    assert_eq!(first.profile, second.profile);
    assert_ne!(first.runtime_instance_id, second.runtime_instance_id);
    assert_ne!(first.load_owner_fence, second.load_owner_fence);
    assert_eq!(
        first.drained_owner_fence.as_deref(),
        Some(first.load_owner_fence.as_str())
    );
    assert_eq!(
        second.drained_owner_fence.as_deref(),
        Some(second.load_owner_fence.as_str())
    );
    assert!(matches!(
        rows[0].identity,
        RuntimeServiceTimingIdentity::Unknown { .. }
    ));
    let old_profile = exact_profile(&rows[1]);
    assert_ne!(old_profile.runtime_instance_id(), first.runtime_instance_id);
    assert!(rows[0]
        .fresh_production_service_interval_ns(&old_profile, &snapshot, 1000)
        .is_none());
    for row in rows.iter() {
        assert!(row
            .fresh_production_history_interval_ns(&first.profile, &snapshot, 1000)
            .is_none());
        assert_eq!(
            protocol_clone(row).fresh_production_history_interval_ns(
                &first.profile,
                &snapshot,
                1000
            ),
            Some(41)
        );
    }
    assert_ne!(
        rows[0].execution_request_id_digest,
        rows[1].execution_request_id_digest
    );
}

#[tokio::test]
async fn history_fresh_drain_fence_or_owner_change_refuses_without_changing_execution() {
    for change in ["fence", "content", "config", "device", "missing"] {
        let (_directory, request, target, decision) = crate::selected_text_execution::fixture();
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        backend.native_owner = Some(owner());
        backend.native_drain_change = Some(change);
        let gateway = instrument(backend, recorder.clone(), clock);
        execute(&gateway, request, target, decision).await.unwrap();
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let rows = recorder.rows.lock().unwrap();
        let history = rows[0].history.as_ref().unwrap();
        assert!(history.drained_owner_fence.is_none(), "{change}");
        assert_eq!(rows[0].outcome, Outcome::Completed);
        assert!(protocol_clone(&rows[0])
            .fresh_production_history_interval_ns(&history.profile, &snapshot, 1000)
            .is_none());
    }
}

#[tokio::test]
async fn history_content_implementation_configuration_device_and_workload_are_distinct() {
    let mut profiles = Vec::new();
    // Compare digest only: separate gateway epochs intentionally remain distinct.
    for change in [
        "same",
        "content",
        "implementation",
        "config",
        "device",
        "input",
    ] {
        let (_directory, mut request, target, decision) = crate::selected_text_execution::fixture();
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        let mut evidence = owner();
        match change {
            "content" => evidence.content_fingerprint = "9".repeat(64),
            "implementation" => evidence.facts.implementation_fingerprint = "9".repeat(64),
            "config" => evidence.facts.effective_configuration_fingerprint = "9".repeat(64),
            "device" => evidence.facts.physical_device_fingerprint = "9".repeat(64),
            "input" => {
                if let crate::InferenceExecutionInput::TextGeneration { system_prompt, .. } =
                    &mut request.input
                {
                    *system_prompt = Some("different exact system workload".into());
                    backend.inner.expected_system_prompt = system_prompt.clone();
                }
            }
            _ => {}
        }
        backend.native_owner = Some(evidence);
        let gateway = instrument(backend, recorder.clone(), clock);
        execute(&gateway, request, target, decision).await.unwrap();
        let rows = recorder.rows.lock().unwrap();
        profiles.push(
            rows[0]
                .history
                .as_ref()
                .unwrap()
                .profile
                .identity_fingerprint()
                .to_owned(),
        );
    }
    profiles.sort();
    profiles.dedup();
    assert_eq!(profiles.len(), 6);
}

#[tokio::test]
async fn history_failed_load_compute_drain_and_unsupported_facts_keep_raw_refusal() {
    for failure in ["load", "compute", "drain", "unsupported"] {
        let (_directory, request, target, decision) = crate::selected_text_execution::fixture();
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        backend.native_owner = Some(owner());
        backend.fail_load = failure == "load";
        backend.fail_compute = failure == "compute";
        backend.fail_cleanup = failure == "drain";
        if failure == "unsupported" {
            backend.native_owner.as_mut().unwrap().facts.device_id = "cuda:0".parse().unwrap();
        }
        let gateway = instrument(backend, recorder.clone(), clock);
        let result = execute(&gateway, request, target, decision).await;
        assert_eq!(result.is_err(), failure != "unsupported");
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let rows = recorder.rows.lock().unwrap();
        assert_eq!(rows.len(), 1);
        if let Some(history) = &rows[0].history {
            assert!(protocol_clone(&rows[0])
                .fresh_production_history_interval_ns(&history.profile, &snapshot, 1000)
                .is_none());
        } else {
            assert!(failure == "load" || failure == "unsupported");
        }
    }
}

/// Explicit native qualification target. Uses only installed Python/Torch and
/// locally generated untrained CPU weights, never downloads or speed assertions.
#[cfg(all(feature = "backend-pytorch", feature = "backend-llamacpp"))]
#[tokio::test]
#[ignore = "requires installed offline Torch/Transformers; run explicitly in a separate process"]
async fn native_history_builtin_gateway_real_reload_text_drain_and_foreign_aba() {
    use pyo3::prelude::*;
    let (directory, mut request, mut target, decision) = crate::selected_text_execution::fixture();
    // Match the synthetic package metadata to the actual locally built bytes.
    let facts = request
        .resolved_model_package_facts
        .as_mut()
        .unwrap()
        .transformers
        .as_mut()
        .unwrap();
    facts.config_model_type = Some("gpt2".into());
    facts.architectures = vec!["GPT2LMHeadModel".into()];
    facts.torch_dtype = Some("float32".into());
    let path = directory.path().join("tiny-native-model");
    let path_string = path.to_str().unwrap().to_owned();
    Python::with_gil(|py| -> PyResult<()> {
        let source = std::ffi::CString::new(r#"
from pathlib import Path
import torch
from tokenizers import Tokenizer, models, pre_tokenizers
from transformers import GPT2Config, GPT2LMHeadModel, PreTrainedTokenizerFast

def build(path):
    torch.set_num_threads(1)
    directory = Path(path)
    directory.mkdir()
    native = Tokenizer(models.WordLevel({"<unk>": 0, "<pad>": 1, "hello": 2, "world": 3}, unk_token="<unk>"))
    native.pre_tokenizer = pre_tokenizers.Whitespace()
    tokenizer = PreTrainedTokenizerFast(tokenizer_object=native, unk_token="<unk>", pad_token="<pad>",
        bos_token="<unk>", eos_token="<pad>", model_max_length=16)
    with torch.random.fork_rng(devices=[]):
        torch.manual_seed(20261009)
        model = GPT2LMHeadModel(GPT2Config(vocab_size=4, n_positions=16, n_ctx=16,
            n_embd=8, n_layer=1, n_head=1, bos_token_id=0, eos_token_id=1, pad_token_id=1))
    model.eval()
    model.save_pretrained(directory, safe_serialization=True)
    tokenizer.save_pretrained(directory)
"#).unwrap();
        let fixture = pyo3::types::PyModule::from_code(py, &source, c"native_history_fixture.py", c"native_history_fixture")?;
        fixture.call_method1("build", (&path_string,))?;
        Ok(())
    }).unwrap();
    target.local_load_path = path_string;
    target.content_fingerprint = None;
    request.generation_options = Some(
        serde_json::from_value(serde_json::json!({
            "length": {"max_new_tokens": 1}
        }))
        .unwrap(),
    );
    let recorder = Arc::new(Recorder::default());
    // Actual built-in constructor + actual monotonic clock; no protocol cloning.
    let gateway =
        Arc::new(InferenceGateway::new().with_runtime_service_timing_recorder(recorder.clone()));
    execute(&gateway, request.clone(), target.clone(), decision.clone())
        .await
        .unwrap();
    request.request_id = Some("second-native-correlation".into());
    execute(&gateway, request.clone(), target.clone(), decision.clone())
        .await
        .unwrap();
    let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
    let rows = recorder.rows.lock().unwrap().clone();
    assert_eq!(rows.len(), 2);
    let first = rows[0]
        .history
        .as_ref()
        .expect("supported installed native owner");
    let second = rows[1].history.as_ref().expect("supported native reload");
    assert_eq!(first.profile, second.profile);
    assert_ne!(first.runtime_instance_id, second.runtime_instance_id);
    assert_ne!(first.load_owner_fence, second.load_owner_fence);
    for row in &rows {
        assert_eq!(
            row.capture.as_ref().unwrap().owner_provenance,
            RuntimeServiceTimingOwnerProvenance::BuiltIn
        );
        assert_eq!(
            row.capture.as_ref().unwrap().load_disposition,
            pantograph_timing_contracts::RuntimeServiceTimingLoadDisposition::Reloaded
        );
        assert!(matches!(
            row.identity,
            RuntimeServiceTimingIdentity::Unknown { .. }
        ));
        assert!(row
            .fresh_production_history_interval_ns(&first.profile, &snapshot, 60_000_000_000)
            .is_some());
    }
    let old = gateway
        .backend
        .read()
        .await
        .runtime_service_timing_attestation()
        .await
        .unwrap();
    // Pause a genuine native collector while its Rust caller is aborted. The
    // physical shutdown must drain it even after the read guard has been dropped.
    let controls = Python::with_gil(|py| -> PyResult<Py<pyo3::types::PyModule>> {
        let source = std::ffi::CString::new(
            r#"
import sys, threading, weakref, gc
owner = sys.modules['pantograph_text_service_timing_owner_v1']
worker = sys.modules['pantograph_torch_worker']
tracker = worker._pantograph_text_timing_tracker_v1
original_snapshot = owner._snapshot
original_wait = tracker._collector_condition.wait
paused = threading.Event()
released = threading.Event()
stop_waiting = threading.Event()
old_model = weakref.ref(worker._model)
def snapshot(*args):
    paused.set()
    if not released.wait(15):
        raise RuntimeError('collector test release missing')
    return original_snapshot(*args)
def wait(*args, **kwargs):
    stop_waiting.set()
    return original_wait(*args, **kwargs)
owner._snapshot = snapshot
tracker._collector_condition.wait = wait
def wait_paused(): return paused.wait(10)
def wait_stop(): return stop_waiting.wait(10)
def release(): released.set()
def restore():
    owner._snapshot = original_snapshot
    tracker._collector_condition.wait = original_wait
    gc.collect()
    return tracker._collectors == 0 and old_model() is None
"#,
        )
        .unwrap();
        Ok(pyo3::types::PyModule::from_code(
            py,
            &source,
            c"native_collector_controls.py",
            c"native_collector_controls",
        )?
        .unbind())
    })
    .unwrap();
    async fn wait_control(controls: &Py<pyo3::types::PyModule>, name: &'static str) {
        let controls = Python::with_gil(|py| controls.clone_ref(py));
        assert!(
            tokio::task::spawn_blocking(move || Python::with_gil(|py| {
                controls
                    .bind(py)
                    .call_method0(name)
                    .unwrap()
                    .extract::<bool>()
                    .unwrap()
            }))
            .await
            .unwrap(),
            "missing collector handshake: {name}"
        );
    }
    let probe_gateway = gateway.clone();
    let probe = tokio::spawn(async move {
        probe_gateway
            .backend
            .read()
            .await
            .runtime_service_timing_attestation()
            .await
    });
    wait_control(&controls, "wait_paused").await;
    probe.abort();
    assert!(probe.await.unwrap_err().is_cancelled());
    let stop_gateway = gateway.clone();
    let stop = tokio::spawn(async move { stop_gateway.stop().await });
    wait_control(&controls, "wait_stop").await;
    assert!(
        !stop.is_finished(),
        "physical ACK preceded the actual collector drain"
    );
    Python::with_gil(|py| {
        controls.bind(py).call_method0("release").unwrap();
    });
    stop.await.unwrap().unwrap();
    assert!(Python::with_gil(|py| controls
        .bind(py)
        .call_method0("restore")
        .unwrap()
        .extract::<bool>()
        .unwrap()));
    execute(&gateway, request, target, decision).await.unwrap();
    // A different actual shared-worker owner performs A -> B -> A. Even the same
    // installed bytes cannot restore this gateway's old load-generation custody.
    let mut foreign = crate::backend::pytorch::PyTorchBackend::new();
    let config = BackendConfig {
        model_path: Some(path.clone()),
        device: Some(BackendStartupDeviceIntent::CanonicalDevice(
            "cpu".parse().unwrap(),
        )),
        ..Default::default()
    };
    foreign
        .start(&config, Arc::new(MockProcessSpawner))
        .await
        .unwrap();
    foreign.unload_model().await.unwrap();
    foreign
        .start(&config, Arc::new(MockProcessSpawner))
        .await
        .unwrap();
    assert!(gateway
        .backend
        .read()
        .await
        .runtime_service_timing_attestation()
        .await
        .is_none());
    assert!(!old.owner_fence.is_empty());
    foreign.stop().await.unwrap();
    gateway.stop().await.unwrap();
}
