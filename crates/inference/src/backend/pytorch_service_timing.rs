//! Opt-in small native CPU text identities. No model-loading or reuse authority.
use crate::{RuntimeServiceTimingOwnerAttestation, RuntimeServiceTimingOwnerFacts};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyString};
use serde::Deserialize;
use std::sync::Arc;

const OWNER_PY: &str = include_str!("../../torch/service_timing_owner.py");

pub(super) struct NativeOwner {
    tracker: Py<PyAny>,
    stamp: Py<PyAny>,
    admission: crate::python_startup_broker::PythonLegacyAdmission,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeFacts {
    owner_fence: String,
    content_fingerprint: String,
    implementation_fingerprint: String,
    effective_configuration_fingerprint: String,
    physical_device_fingerprint: String,
    device_id: crate::InferenceDeviceId,
}

fn decode(value: &Bound<'_, PyAny>) -> Option<RuntimeServiceTimingOwnerAttestation> {
    let string = value.downcast::<PyString>().ok()?;
    let text = string.to_str().ok()?;
    if text.len() > 2048 {
        return None;
    }
    let facts: NativeFacts = serde_json::from_str(text).ok()?;
    let attestation = RuntimeServiceTimingOwnerAttestation {
        owner_fence: facts.owner_fence,
        content_fingerprint: facts.content_fingerprint,
        facts: RuntimeServiceTimingOwnerFacts {
            implementation_fingerprint: facts.implementation_fingerprint,
            effective_configuration_fingerprint: facts.effective_configuration_fingerprint,
            physical_device_fingerprint: facts.physical_device_fingerprint,
            device_id: facts.device_id,
        },
    };
    attestation.valid().then_some(attestation)
}

fn tracker<'py>(py: Python<'py>, worker: &Bound<'py, PyModule>) -> Option<Bound<'py, PyAny>> {
    let sys = py.import("sys").ok()?;
    let modules = sys.getattr("modules").ok()?;
    let name = "pantograph_text_service_timing_owner_v1";
    let module = match modules.get_item(name) {
        Ok(module) => module,
        Err(_) => {
            let source = std::ffi::CString::new(OWNER_PY).ok()?;
            PyModule::from_code(
                py,
                &source,
                c"service_timing_owner.py",
                c"pantograph_text_service_timing_owner_v1",
            )
            .ok()?
            .into_any()
        }
    };
    let implementation = super::pytorch_worker::text_timing_implementation_digest(OWNER_PY);
    let tracker = module
        .call_method1("install", (worker, implementation))
        .ok()?;
    (!tracker.is_none()).then_some(tracker)
}

/// Only the original load response may fail execution. Instrumentation setup,
/// unknown profiles or invalid stamps quietly refuse evidence. Never retry a
/// load after an effectful call, and never attach a later current-owner lookup.
pub(super) fn load_with_ack(
    py: Python<'_>,
    worker: &Bound<'_, PyModule>,
    envelope: String,
    admission: &crate::python_startup_broker::PythonLegacyAdmission,
) -> PyResult<(String, Option<Arc<NativeOwner>>)> {
    let Some(tracker) = tracker(py, worker) else {
        return worker
            .call_method1("load_transformers_model_from_envelope", (envelope,))?
            .extract::<String>()
            .map(|response| (response, None));
    };
    let result = tracker.call_method1("load_with_ack", (envelope,))?;
    let (response, stamp): (String, Py<PyAny>) = result.extract()?;
    let owner = if stamp.bind(py).is_none() {
        None
    } else {
        let facts = stamp.bind(py).getattr("facts_json").ok();
        facts.and_then(|facts| decode(&facts)).map(|_| {
            Arc::new(NativeOwner {
                tracker: tracker.unbind(),
                stamp,
                admission: admission.clone(),
            })
        })
    };
    Ok((response, owner))
}

pub(super) async fn revalidate(
    owner: Arc<NativeOwner>,
) -> Option<RuntimeServiceTimingOwnerAttestation> {
    tokio::task::spawn_blocking(move || {
        owner.admission.with_gil(|py| {
            let value = owner
                .tracker
                .bind(py)
                .call_method1("revalidate", (owner.stamp.bind(py),))
                .ok()?;
            decode(&value)
        })
    })
    .await
    .ok()
    .flatten()
}
