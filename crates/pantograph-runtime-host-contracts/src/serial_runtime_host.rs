//! Explicit in-process serial-owner capability. Default execution ports do not
//! gain this capability or scheduling behavior from a timing label.
use crate::{
    ReservationLifecycleApplication, ReservationLifecycleEvent, ReservationLifecyclePort,
    ReservationLifecyclePortError, RuntimeHostBatchExecutionPort, RuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionResponse, RuntimeHostExecutionCancellationHandle,
    RuntimeHostExecutionInput, RuntimeHostExecutionInputValue, RuntimeHostExecutionPortError,
};
use async_trait::async_trait;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerSerialBoundDispatch, SchedulerSerialDrainedDispatch,
    SchedulerSerialOwnerSnapshot,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

const MAX_PREPARATION_METADATA_BYTES: usize = 32 * 1024;

/// Actual Ready task data for load-only preparation before candidate admission.
///
/// The correlation ID identifies preparation, not a Started task attempt. This
/// request carries no candidate, execution lease, handoff or public cancellation
/// context. The port owns its native preparation/shutdown signal. Workflow
/// readiness and prepared-owner evidence must still be fenced at final Start.
#[derive(Debug)]
pub struct SerialRuntimeHostPreparationRequest {
    pub preparation_request_id: String,
    pub task_intent: SchedulableTaskIntent,
    pub materialized_inputs: Vec<RuntimeHostExecutionInput>,
}

impl SerialRuntimeHostPreparationRequest {
    /// Validate borrowed metadata and actual input before copying or native work.
    pub fn new(
        preparation_request_id: &str,
        task_intent: &SchedulableTaskIntent,
        materialized_inputs: &[RuntimeHostExecutionInput],
    ) -> Result<Self, RuntimeHostExecutionPortError> {
        validate_preparation(preparation_request_id, task_intent, materialized_inputs)?;
        Ok(Self {
            preparation_request_id: preparation_request_id.to_owned(),
            task_intent: task_intent.clone(),
            materialized_inputs: materialized_inputs.to_vec(),
        })
    }

    /// Recheck the public fields before preparation, including old-owner stop.
    pub fn validate(&self) -> Result<(), RuntimeHostExecutionPortError> {
        validate_preparation(
            &self.preparation_request_id,
            &self.task_intent,
            &self.materialized_inputs,
        )
    }
}

fn preparation_error(message: &str) -> RuntimeHostExecutionPortError {
    RuntimeHostExecutionPortError::ExecutionFailed {
        message: message.to_owned(),
    }
}

fn validate_preparation(
    id: &str,
    intent: &SchedulableTaskIntent,
    inputs: &[RuntimeHostExecutionInput],
) -> Result<(), RuntimeHostExecutionPortError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err(preparation_error("invalid serial CPU preparation identity"));
    }
    if intent.task_type.as_str() != "embedding"
        || intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .map(|id| id.as_str())
            != Some("candle")
        || intent
            .constraints
            .requested_device_id
            .as_ref()
            .map(|id| id.as_str())
            != Some("cpu")
        || !intent.trait_settings.is_empty()
        || !intent.dependency_override_patches.is_empty()
        || intent.estimate_hints.len() > 32
    {
        return Err(preparation_error("serial CPU preparation requires bounded embedding intent on candle/cpu without overrides"));
    }
    let model = &intent.model_ref;
    if model.model_id.len() > 128
        || model
            .revision
            .as_ref()
            .is_some_and(|value| value.len() > 128)
        || model
            .selected_artifact_id
            .as_ref()
            .is_some_and(|value| value.len() > 128)
        || model
            .selected_artifact_path
            .as_ref()
            .is_some_and(|value| value.len() > 1024)
        || model.migration_diagnostics.len() > 4
    {
        return Err(preparation_error(
            "serial CPU preparation model metadata is not bounded",
        ));
    }
    let [input] = inputs else {
        return Err(preparation_error(
            "serial CPU preparation requires one actual text input",
        ));
    };
    let text = match &input.value {
        RuntimeHostExecutionInputValue::String(text)
        | RuntimeHostExecutionInputValue::TranscriptText(text) => text,
        _ => {
            return Err(preparation_error(
                "serial CPU preparation requires an actual text input",
            ))
        }
    };
    if input.port_id != "text" || text.len() > 4096 || text.trim().is_empty() {
        return Err(preparation_error(
            "serial CPU preparation text must be nonblank and at most 4096 bytes",
        ));
    }
    // Bound borrowed serialization before validating/cloning variable metadata.
    // The writer counts bytes without allocating a serialized metadata buffer.
    let mut budget = PreparationMetadataBudget(MAX_PREPARATION_METADATA_BYTES);
    serde_json::to_writer(&mut budget, &(id, intent, inputs)).map_err(|_| {
        preparation_error("serial CPU preparation metadata exceeds its bounded budget")
    })?;
    intent
        .validate()
        .map_err(|_| preparation_error("invalid serial CPU preparation intent"))?;
    Ok(())
}

struct PreparationMetadataBudget(usize);
impl std::io::Write for PreparationMetadataBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.checked_sub(bytes.len()).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "preparation metadata bound",
            )
        })?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[async_trait]
pub trait SerialRuntimeHostBatchExecutionPort:
    RuntimeHostBatchExecutionPort + ReservationLifecyclePort
{
    /// Prepare only this actual Ready CPU task under its granted service turn.
    ///
    /// Implementations validate the request before Pumas, stop or load work, own
    /// their native cancellation/shutdown signal, and retain linear residency
    /// authority privately. Returned owner evidence grants no execution or cleanup
    /// authority. Existing retained-only ports explicitly refuse cold preparation.
    async fn prepare_serial_cpu(
        &self,
        _request: SerialRuntimeHostPreparationRequest,
    ) -> Result<SerialRuntimeHostCpuOwnerEvidence, RuntimeHostExecutionPortError> {
        Err(preparation_error(
            "serial CPU residency preparation is unsupported by this port",
        ))
    }

    /// Read-only nonblocking query on the exact admitted CPU model/config. None
    /// means baseline order, not permission to fabricate an owner/drain proof.
    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence>;

    /// Exactly one member. The actual execution owner consumes the bound guard
    /// inside its held backend lease and returns the drain proof only after
    /// actual worker completion. Return collection evidence only from that actually
    /// held owner, rechecked after drain (None keeps baseline order). Expected-owner mismatch refuses before forward;
    /// never downgrade a ranked attempt to unranked execution after Start.
    async fn execute_serial_singleton(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<
        (
            RuntimeHostBatchExecutionResponse,
            SchedulerSerialDrainedDispatch,
            Option<SerialRuntimeHostCpuOwnerEvidence>,
        ),
        RuntimeHostExecutionPortError,
    >;

    /// Additive linear cleanup path. Legacy ports retain exact-owner semantics.
    async fn execute_serial_singleton_with_cleanup(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<SerialRuntimeHostDrainedExecution, RuntimeHostExecutionPortError> {
        let (response, drained, owner) = self
            .execute_serial_singleton(request, cancellation, ownership)
            .await?;
        Ok(SerialRuntimeHostDrainedExecution {
            response,
            drained,
            owner,
            cleanup: None,
        })
    }

    /// Apply the real owned release/reconciliation. For qualified warm attempts,
    /// release must atomically fence this expected loaded owner or exclude every
    /// relevant producer entry path through cleanup. Failed fencing returns Err;
    /// it cannot acknowledge or stop a replacement producer.
    async fn apply_serial_cleanup(
        &self,
        event: ReservationLifecycleEvent,
        expected_owner: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError>;
}

/// Issued by the constructor-owned CPU owner. Generation reads are atomic and
/// safe under the scheduler store lock; no callback executes at final Start.
#[derive(Clone)]
pub struct SerialRuntimeHostCpuOwnerEvidence {
    pub snapshot: SchedulerSerialOwnerSnapshot,
    pub generation: Arc<AtomicU64>,
}
impl SerialRuntimeHostCpuOwnerEvidence {
    pub fn is_current(&self) -> bool {
        self.snapshot.generation > 0
            && self.snapshot.generation.is_multiple_of(2)
            && self.generation.load(Ordering::Acquire) == self.snapshot.generation
    }
}

/// In-process linear handoff. No serialization, Clone, global receipt map or
/// caller replacement owner is provided. Implement on the actual native receipt.
#[async_trait]
pub trait SerialRuntimeHostCleanupReceipt: Send {
    fn previous_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence;
    fn current_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence;
    async fn apply(
        self: Box<Self>,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError>;
}
#[must_use]
pub struct SerialRuntimeHostDrainedExecution {
    pub response: RuntimeHostBatchExecutionResponse,
    pub drained: SchedulerSerialDrainedDispatch,
    pub owner: Option<SerialRuntimeHostCpuOwnerEvidence>,
    pub cleanup: Option<Box<dyn SerialRuntimeHostCleanupReceipt>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn intent() -> SchedulableTaskIntent {
        serde_json::from_value(json!({
            "workflow_id":"wf.cpu","workflow_run_id":"run.cpu","node_id":"infer","task_id":"task.cpu",
            "task_type":"embedding","model_ref":{"model_id":"embedding/test","selected_artifact_id":"main"},
            "constraints":{"requested_runtime_id":"candle","requested_device_id":"cpu"}
        })).unwrap()
    }

    fn inputs(text: &str) -> Vec<RuntimeHostExecutionInput> {
        vec![RuntimeHostExecutionInput {
            port_id: "text".into(),
            value: RuntimeHostExecutionInputValue::String(text.into()),
        }]
    }

    #[test]
    fn preparation_copies_only_valid_actual_inputs_and_rechecks_public_mutation() {
        let intent = intent();
        let inputs = inputs("actual Ready text");
        let mut request =
            SerialRuntimeHostPreparationRequest::new("prepare.1", &intent, &inputs).unwrap();
        assert_eq!(request.task_intent, intent);
        assert_eq!(request.materialized_inputs, inputs);
        request.materialized_inputs[0].port_id = "placeholder".into();
        assert!(request.validate().is_err());
    }

    #[test]
    fn preparation_enforces_text_byte_limit_and_actual_single_input_shape() {
        let intent = intent();
        assert!(SerialRuntimeHostPreparationRequest::new(
            "prepare.1",
            &intent,
            &inputs(&"é".repeat(2048))
        )
        .is_ok());
        for text in [String::new(), " \t\n".into(), "é".repeat(2049)] {
            assert!(
                SerialRuntimeHostPreparationRequest::new("prepare.1", &intent, &inputs(&text))
                    .is_err()
            );
        }
        assert!(SerialRuntimeHostPreparationRequest::new("prepare.1", &intent, &[]).is_err());
        let mut two = inputs("hello");
        two.extend(inputs("world"));
        assert!(SerialRuntimeHostPreparationRequest::new("prepare.1", &intent, &two).is_err());
        let wrong_value = vec![RuntimeHostExecutionInput {
            port_id: "text".into(),
            value: RuntimeHostExecutionInputValue::Bool(true),
        }];
        assert!(
            SerialRuntimeHostPreparationRequest::new("prepare.1", &intent, &wrong_value).is_err()
        );
    }

    #[test]
    fn preparation_refuses_foreign_runtime_missing_cpu_and_trait_overrides() {
        for (field, value) in [
            ("task_type", json!("text_generation")),
            (
                "constraints",
                json!({"requested_runtime_id":"candle","requested_device_id":"cuda:0"}),
            ),
            ("constraints", json!({"requested_runtime_id":"candle"})),
            ("constraints", json!({"requested_device_id":"cpu"})),
            (
                "constraints",
                json!({"requested_runtime_id":"llamacpp","requested_device_id":"cpu"}),
            ),
            (
                "trait_settings",
                json!([{"trait_id":"temperature","value":{"kind":"u64","value":1}}]),
            ),
            ("contract_version", json!(99)),
        ] {
            let mut value_intent = serde_json::to_value(intent()).unwrap();
            value_intent[field] = value;
            let foreign: SchedulableTaskIntent = serde_json::from_value(value_intent).unwrap();
            assert!(
                SerialRuntimeHostPreparationRequest::new("prepare.1", &foreign, &inputs("hello"))
                    .is_err(),
                "accepted {field}"
            );
        }
    }

    #[test]
    fn preparation_bounds_identity_and_serialized_metadata_before_copying() {
        let intent = intent();
        for id in [
            String::new(),
            "x".repeat(129),
            " bad id ".into(),
            "prepare/foreign".into(),
        ] {
            assert!(
                SerialRuntimeHostPreparationRequest::new(&id, &intent, &inputs("hello")).is_err()
            );
        }
        let mut huge = serde_json::to_value(&intent).unwrap();
        huge["model_ref"]["migration_diagnostics"] =
            json!([{"code":"legacy","message":"x".repeat(40000),"input":"old"}]);
        let huge: SchedulableTaskIntent = serde_json::from_value(huge).unwrap();
        assert!(
            SerialRuntimeHostPreparationRequest::new("prepare.1", &huge, &inputs("hello")).is_err()
        );
        let mut huge_id = intent;
        huge_id.model_ref.model_id = "x".repeat(129);
        assert!(
            SerialRuntimeHostPreparationRequest::new("prepare.1", &huge_id, &inputs("hello"))
                .is_err()
        );
    }
}
