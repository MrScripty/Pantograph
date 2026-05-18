use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::{
    unavailable_peak_ram_observation, RuntimeResourceMonitor, RuntimeResourceMonitorError,
    RuntimeResourceMonitorGuard, PROCESS_RSS_MONITOR_THREAD_NAME, PROCESS_RSS_SAMPLE_INTERVAL,
};
use crate::{
    InferenceExecutionResourceObservation, InferenceResourceObservationSourceKind,
    InferenceResourceObservationUnavailableState,
};

/// Process RSS monitor implemented with the existing `sysinfo` dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessRssResourceMonitor {
    sample_interval: Duration,
}

impl ProcessRssResourceMonitor {
    /// Construct a monitor with a custom sampling interval.
    pub fn with_sample_interval(
        sample_interval: Duration,
    ) -> Result<Self, RuntimeResourceMonitorError> {
        if sample_interval.is_zero() {
            return Err(RuntimeResourceMonitorError::InvalidSampleInterval);
        }

        Ok(Self { sample_interval })
    }
}

impl Default for ProcessRssResourceMonitor {
    fn default() -> Self {
        Self {
            sample_interval: PROCESS_RSS_SAMPLE_INTERVAL,
        }
    }
}

impl RuntimeResourceMonitor for ProcessRssResourceMonitor {
    fn start_process_monitor(
        &self,
        process_id: u32,
    ) -> Result<RuntimeResourceMonitorGuard, RuntimeResourceMonitorError> {
        Ok(RuntimeResourceMonitorGuard::process_rss(
            ProcessRssMonitorGuard::start(process_id, self.sample_interval)?,
        ))
    }
}

/// Owns one process RSS sampler.
#[derive(Debug)]
#[must_use]
pub struct ProcessRssMonitorGuard {
    stop_requested: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<ProcessRssMonitorState>>,
}

impl ProcessRssMonitorGuard {
    fn start(
        process_id: u32,
        sample_interval: Duration,
    ) -> Result<Self, RuntimeResourceMonitorError> {
        if process_id == 0 {
            return Err(RuntimeResourceMonitorError::InvalidProcessId);
        }

        let stop_requested = Arc::new(AtomicBool::new(false));
        let thread_stop_requested = Arc::clone(&stop_requested);
        let join_handle = thread::Builder::new()
            .name(PROCESS_RSS_MONITOR_THREAD_NAME.to_string())
            .spawn(move || sample_process_rss(process_id, sample_interval, thread_stop_requested))
            .map_err(|error| RuntimeResourceMonitorError::ThreadSpawnFailed {
                message: error.to_string(),
            })?;

        Ok(Self {
            stop_requested,
            join_handle: Some(join_handle),
        })
    }

    /// Stop the sampler and return the observed process RSS peak.
    pub fn finish(
        mut self,
    ) -> Result<InferenceExecutionResourceObservation, RuntimeResourceMonitorError> {
        self.finish_inner()
    }

    fn finish_inner(
        &mut self,
    ) -> Result<InferenceExecutionResourceObservation, RuntimeResourceMonitorError> {
        self.stop_requested.store(true, Ordering::Release);
        let Some(join_handle) = self.join_handle.take() else {
            return unavailable_peak_ram_observation(
                InferenceResourceObservationUnavailableState::NotAvailable,
            );
        };

        let state = join_handle
            .join()
            .map_err(|_| RuntimeResourceMonitorError::MonitorThreadPanicked)?;
        state.into_observation()
    }
}

impl Drop for ProcessRssMonitorGuard {
    fn drop(&mut self) {
        self.stop_requested.store(true, Ordering::Release);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessRssMonitorState {
    peak_ram_bytes: Option<u64>,
}

impl ProcessRssMonitorState {
    fn into_observation(
        self,
    ) -> Result<InferenceExecutionResourceObservation, RuntimeResourceMonitorError> {
        match self.peak_ram_bytes {
            Some(peak_ram_bytes) => Ok(InferenceExecutionResourceObservation::peak_ram(
                peak_ram_bytes,
                InferenceResourceObservationSourceKind::OsProcessRss,
            )?),
            None => unavailable_peak_ram_observation(
                InferenceResourceObservationUnavailableState::NotAvailable,
            ),
        }
    }
}

fn sample_process_rss(
    process_id: u32,
    sample_interval: Duration,
    stop_requested: Arc<AtomicBool>,
) -> ProcessRssMonitorState {
    let mut system = System::new();
    let target_pid = Pid::from_u32(process_id);
    let mut peak_ram_bytes = None;

    loop {
        peak_ram_bytes = max_optional_u64(
            peak_ram_bytes,
            observe_process_rss_bytes(&mut system, target_pid),
        );

        if stop_requested.load(Ordering::Acquire) {
            break;
        }

        thread::sleep(sample_interval);
    }

    peak_ram_bytes = max_optional_u64(
        peak_ram_bytes,
        observe_process_rss_bytes(&mut system, target_pid),
    );

    ProcessRssMonitorState { peak_ram_bytes }
}

fn observe_process_rss_bytes(system: &mut System, target_pid: Pid) -> Option<u64> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target_pid]),
        true,
        ProcessRefreshKind::new().with_memory(),
    );

    system
        .process(target_pid)
        .map(sysinfo::Process::memory)
        .filter(|value| *value > 0)
}

fn max_optional_u64(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}
