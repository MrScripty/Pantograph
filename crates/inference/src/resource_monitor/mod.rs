//! Runtime execution resource monitors.
//!
//! This module provides platform-neutral monitor lifecycle contracts. It emits
//! typed resource observations only; it does not own scheduler policy,
//! diagnostic persistence, or runtime candidate ranking.

use std::time::Duration;

use thiserror::Error;

use crate::{
    InferenceExecutionResourceObservation, InferenceExecutionResourceObservationError,
    InferenceResourceObservationAvailability, InferenceResourceObservationMetricKind,
    InferenceResourceObservationSourceKind, InferenceResourceObservationUnavailableState,
};

mod platform;
pub mod process_rss;
pub mod unsupported;

/// Default process RSS sampling interval.
pub const PROCESS_RSS_SAMPLE_INTERVAL: Duration = Duration::from_millis(100);

/// Name used for the process RSS sampler thread.
pub const PROCESS_RSS_MONITOR_THREAD_NAME: &str = "inference-process-rss-monitor";

/// Resource monitor contract for one runtime execution boundary.
pub trait RuntimeResourceMonitor: Send + Sync {
    /// Start observing resources for the process.
    fn start_process_monitor(
        &self,
        process_id: u32,
    ) -> Result<RuntimeResourceMonitorGuard, RuntimeResourceMonitorError>;
}

/// Return the default resource monitor for the current platform.
///
/// Supported platforms currently use process RSS through `sysinfo`; unsupported
/// targets return explicit unavailable observations instead of fake values.
pub fn default_runtime_resource_monitor() -> impl RuntimeResourceMonitor {
    platform::default_runtime_resource_monitor()
}

/// Owns one active resource monitor.
#[derive(Debug)]
#[must_use]
pub struct RuntimeResourceMonitorGuard {
    inner: RuntimeResourceMonitorGuardInner,
}

impl RuntimeResourceMonitorGuard {
    pub(crate) fn process_rss(guard: process_rss::ProcessRssMonitorGuard) -> Self {
        Self {
            inner: RuntimeResourceMonitorGuardInner::ProcessRss(guard),
        }
    }

    pub(crate) fn unsupported(guard: unsupported::UnsupportedRuntimeResourceMonitorGuard) -> Self {
        Self {
            inner: RuntimeResourceMonitorGuardInner::Unsupported(guard),
        }
    }

    /// Stop observing and return the collected resource observation.
    pub fn finish(
        self,
    ) -> Result<InferenceExecutionResourceObservation, RuntimeResourceMonitorError> {
        match self.inner {
            RuntimeResourceMonitorGuardInner::ProcessRss(guard) => guard.finish(),
            RuntimeResourceMonitorGuardInner::Unsupported(guard) => guard.finish(),
        }
    }
}

#[derive(Debug)]
enum RuntimeResourceMonitorGuardInner {
    ProcessRss(process_rss::ProcessRssMonitorGuard),
    Unsupported(unsupported::UnsupportedRuntimeResourceMonitorGuard),
}

/// Error returned by resource monitor lifecycle operations.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeResourceMonitorError {
    #[error("process id must be non-zero")]
    InvalidProcessId,
    #[error("process RSS sample interval must be greater than zero")]
    InvalidSampleInterval,
    #[error("failed to spawn process RSS monitor thread: {message}")]
    ThreadSpawnFailed { message: String },
    #[error("process RSS monitor thread panicked")]
    MonitorThreadPanicked,
    #[error(transparent)]
    InvalidObservation(#[from] InferenceExecutionResourceObservationError),
}

fn unavailable_peak_ram_observation(
    state: InferenceResourceObservationUnavailableState,
) -> Result<InferenceExecutionResourceObservation, RuntimeResourceMonitorError> {
    Ok(InferenceExecutionResourceObservation::unavailable(vec![
        InferenceResourceObservationAvailability::new(
            InferenceResourceObservationMetricKind::PeakRamBytes,
            state,
            Some(InferenceResourceObservationSourceKind::OsProcessRss),
        ),
    ])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InferenceResourceObservationMetricKind, InferenceResourceObservationSourceKind,
        InferenceResourceObservationUnavailableState,
    };

    #[test]
    fn resource_monitor_rejects_zero_process_id() {
        let monitor = process_rss::ProcessRssResourceMonitor::default();

        let error = monitor
            .start_process_monitor(0)
            .expect_err("zero process id should fail");

        assert_eq!(error, RuntimeResourceMonitorError::InvalidProcessId);
    }

    #[test]
    fn process_rss_monitor_rejects_zero_sample_interval() {
        let error = process_rss::ProcessRssResourceMonitor::with_sample_interval(Duration::ZERO)
            .expect_err("zero sample interval should fail");

        assert_eq!(error, RuntimeResourceMonitorError::InvalidSampleInterval);
    }

    #[test]
    fn unsupported_resource_monitor_returns_typed_unavailable_observation() {
        let monitor = unsupported::UnsupportedRuntimeResourceMonitor::default();
        let guard = monitor
            .start_process_monitor(std::process::id())
            .expect("unsupported monitor starts");

        let observation = guard.finish().expect("unsupported monitor finishes");

        assert_eq!(observation.peak_ram_bytes(), None);
        assert_eq!(observation.availability().len(), 1);
        assert_eq!(
            observation.availability()[0].metric_kind(),
            InferenceResourceObservationMetricKind::PeakRamBytes
        );
        assert_eq!(
            observation.availability()[0].state(),
            InferenceResourceObservationUnavailableState::UnsupportedPlatform
        );
        assert_eq!(
            observation.availability()[0].source_kind(),
            Some(InferenceResourceObservationSourceKind::OsProcessRss)
        );
    }

    #[test]
    fn default_resource_monitor_reports_peak_or_typed_unavailability() {
        let monitor = default_runtime_resource_monitor();
        let guard = monitor
            .start_process_monitor(std::process::id())
            .expect("default monitor starts");

        std::thread::sleep(PROCESS_RSS_SAMPLE_INTERVAL + PROCESS_RSS_SAMPLE_INTERVAL);

        let observation = guard.finish().expect("default monitor finishes");

        assert!(
            observation.peak_ram_bytes().is_some() || !observation.availability().is_empty(),
            "monitor must report a value or an explicit availability fact"
        );
    }

    #[test]
    fn process_rss_monitor_reports_current_process_peak_ram() {
        let monitor =
            process_rss::ProcessRssResourceMonitor::with_sample_interval(Duration::from_millis(10))
                .expect("valid sample interval");
        let guard = monitor
            .start_process_monitor(std::process::id())
            .expect("process RSS monitor starts");

        std::thread::sleep(Duration::from_millis(30));

        let observation = guard.finish().expect("process RSS monitor finishes");

        assert!(matches!(observation.peak_ram_bytes(), Some(value) if value > 0));
        assert_eq!(observation.sources().len(), 1);
        assert_eq!(
            observation.sources()[0].source_kind(),
            InferenceResourceObservationSourceKind::OsProcessRss
        );
        assert_eq!(
            observation.sources()[0].metric_kind(),
            InferenceResourceObservationMetricKind::PeakRamBytes
        );
    }

    #[test]
    fn process_rss_monitor_drop_stops_sampler_without_finish() {
        let monitor =
            process_rss::ProcessRssResourceMonitor::with_sample_interval(Duration::from_millis(1))
                .expect("valid sample interval");
        let guard = monitor
            .start_process_monitor(std::process::id())
            .expect("process RSS monitor starts");

        drop(guard);
    }
}
