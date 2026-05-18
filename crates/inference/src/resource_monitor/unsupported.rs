use super::{
    unavailable_peak_ram_observation, RuntimeResourceMonitor, RuntimeResourceMonitorError,
    RuntimeResourceMonitorGuard,
};
use crate::{InferenceExecutionResourceObservation, InferenceResourceObservationUnavailableState};

/// Resource monitor used when the target platform has no supported collector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnsupportedRuntimeResourceMonitor;

impl RuntimeResourceMonitor for UnsupportedRuntimeResourceMonitor {
    fn start_process_monitor(
        &self,
        process_id: u32,
    ) -> Result<RuntimeResourceMonitorGuard, RuntimeResourceMonitorError> {
        if process_id == 0 {
            return Err(RuntimeResourceMonitorError::InvalidProcessId);
        }

        Ok(RuntimeResourceMonitorGuard::unsupported(
            UnsupportedRuntimeResourceMonitorGuard,
        ))
    }
}

/// Finished immediately because unsupported targets do not start a sampler.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[must_use]
pub struct UnsupportedRuntimeResourceMonitorGuard;

impl UnsupportedRuntimeResourceMonitorGuard {
    pub fn finish(
        self,
    ) -> Result<InferenceExecutionResourceObservation, RuntimeResourceMonitorError> {
        unavailable_peak_ram_observation(
            InferenceResourceObservationUnavailableState::UnsupportedPlatform,
        )
    }
}
