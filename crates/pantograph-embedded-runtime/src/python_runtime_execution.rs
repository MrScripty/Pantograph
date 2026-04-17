use crate::runtime_health::RuntimeHealthAssessment;
use std::sync::Mutex;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PythonRuntimeExecutionMetadata {
    pub snapshot: inference::RuntimeLifecycleSnapshot,
    pub model_target: Option<String>,
    pub health_assessment: Option<RuntimeHealthAssessment>,
}

#[derive(Debug, Default)]
pub struct PythonRuntimeExecutionRecorder {
    state: Mutex<Vec<PythonRuntimeExecutionMetadata>>,
}

impl PythonRuntimeExecutionRecorder {
    pub fn record(&self, metadata: PythonRuntimeExecutionMetadata) {
        self.state
            .lock()
            .expect("python runtime recorder lock")
            .push(metadata);
    }

    pub fn previous_consecutive_failures(&self, runtime_instance_id: Option<&str>) -> u32 {
        let Some(runtime_instance_id) = runtime_instance_id else {
            return 0;
        };

        self.state
            .lock()
            .expect("python runtime recorder lock")
            .iter()
            .rev()
            .find(|metadata| {
                metadata.snapshot.runtime_instance_id.as_deref() == Some(runtime_instance_id)
            })
            .and_then(|metadata| metadata.health_assessment.as_ref())
            .map(|assessment| assessment.consecutive_failures)
            .unwrap_or(0)
    }

    pub fn snapshot(&self) -> Option<PythonRuntimeExecutionMetadata> {
        self.state
            .lock()
            .expect("python runtime recorder lock")
            .last()
            .cloned()
    }

    pub fn snapshots(&self) -> Vec<PythonRuntimeExecutionMetadata> {
        self.state
            .lock()
            .expect("python runtime recorder lock")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime_health::{failed_runtime_health_assessment, RuntimeHealthState};

    use super::*;

    #[test]
    fn previous_consecutive_failures_uses_latest_matching_runtime_instance() {
        let recorder = PythonRuntimeExecutionRecorder::default();
        recorder.record(PythonRuntimeExecutionMetadata {
            snapshot: inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                ..inference::RuntimeLifecycleSnapshot::default()
            },
            model_target: Some("/models/one.safetensors".to_string()),
            health_assessment: Some(failed_runtime_health_assessment("crash-1", 0, 3)),
        });
        recorder.record(PythonRuntimeExecutionMetadata {
            snapshot: inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:other".to_string()),
                ..inference::RuntimeLifecycleSnapshot::default()
            },
            model_target: Some("/models/two.safetensors".to_string()),
            health_assessment: Some(failed_runtime_health_assessment("crash-2", 1, 3)),
        });
        recorder.record(PythonRuntimeExecutionMetadata {
            snapshot: inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                ..inference::RuntimeLifecycleSnapshot::default()
            },
            model_target: Some("/models/one.safetensors".to_string()),
            health_assessment: None,
        });

        assert_eq!(
            recorder.previous_consecutive_failures(Some("python-runtime:pytorch:default")),
            0
        );
        assert_eq!(
            recorder.previous_consecutive_failures(Some("python-runtime:pytorch:other")),
            2
        );
        assert_eq!(recorder.previous_consecutive_failures(None), 0);

        let latest_other = recorder
            .snapshots()
            .into_iter()
            .find(|metadata| {
                metadata.snapshot.runtime_instance_id.as_deref()
                    == Some("python-runtime:pytorch:other")
            })
            .and_then(|metadata| metadata.health_assessment)
            .expect("other runtime assessment");
        assert_eq!(
            latest_other.state,
            RuntimeHealthState::Degraded {
                reason: "crash-2".to_string(),
            }
        );
    }
}
