use crate::state::RuntimeRegistryStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeObservation {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub model_id: Option<String>,
    pub status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub last_error: Option<String>,
}

pub fn observed_runtime_status_from_lifecycle(
    active: bool,
    warmup_started_at_ms: Option<u64>,
    warmup_completed_at_ms: Option<u64>,
    has_error: bool,
) -> RuntimeRegistryStatus {
    // An effectful start is not active until it completes. Its unfinished
    // marker still represents an owned transition, including an abandoned
    // caller; do not let reconciliation authorize another start.
    if warmup_started_at_ms.is_some() && warmup_completed_at_ms.is_none() && !has_error {
        return RuntimeRegistryStatus::Warming;
    }
    if active {
        if warmup_started_at_ms.is_some() && warmup_completed_at_ms.is_none() {
            return RuntimeRegistryStatus::Warming;
        }

        return RuntimeRegistryStatus::Ready;
    }

    if has_error {
        return RuntimeRegistryStatus::Failed;
    }

    RuntimeRegistryStatus::Stopped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_runtime_status_marks_inactive_unfinished_warmup_as_warming() {
        assert_eq!(
            observed_runtime_status_from_lifecycle(false, Some(10), None, false),
            RuntimeRegistryStatus::Warming
        );
        assert_eq!(
            observed_runtime_status_from_lifecycle(false, Some(10), None, true),
            RuntimeRegistryStatus::Failed
        );
    }

    #[test]
    fn observed_runtime_status_marks_active_warmup_as_warming() {
        assert_eq!(
            observed_runtime_status_from_lifecycle(true, Some(10), None, false),
            RuntimeRegistryStatus::Warming
        );
    }

    #[test]
    fn observed_runtime_status_marks_active_completed_warmup_as_ready() {
        assert_eq!(
            observed_runtime_status_from_lifecycle(true, Some(10), Some(20), false),
            RuntimeRegistryStatus::Ready
        );
    }

    #[test]
    fn observed_runtime_status_marks_inactive_error_as_failed() {
        assert_eq!(
            observed_runtime_status_from_lifecycle(false, None, None, true),
            RuntimeRegistryStatus::Failed
        );
    }

    #[test]
    fn observed_runtime_status_marks_inactive_error_free_runtime_as_stopped() {
        assert_eq!(
            observed_runtime_status_from_lifecycle(false, None, None, false),
            RuntimeRegistryStatus::Stopped
        );
    }
}
