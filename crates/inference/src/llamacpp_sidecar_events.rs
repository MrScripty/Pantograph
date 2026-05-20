//! Typed llama.cpp sidecar event classification.
//!
//! This module is the adapter-local boundary for interpreting bounded
//! llama.cpp process output. Generic process, gateway, scheduler, and
//! diagnostics code must consume the typed result instead of parsing logs.

use thiserror::Error;

use crate::process::ProcessEvent;
use crate::InferenceMemoryFailureKind;

/// Typed sidecar startup/readiness failure.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum LlamaCppSidecarStartupError {
    #[error("llama.cpp sidecar startup failed: managed binary error: {message}")]
    ManagedBinary { message: String },
    #[error("llama.cpp sidecar startup failed: out of memory")]
    OutOfMemory,
    #[error("llama.cpp sidecar startup failed: process error")]
    ProcessError,
    #[error("llama.cpp sidecar startup failed: process terminated unexpectedly")]
    ProcessTerminated { status: Option<i32> },
    #[error("llama.cpp sidecar startup failed: readiness timeout")]
    ReadinessTimeout,
    #[error("llama.cpp sidecar startup failed: HTTP readiness check failed")]
    HttpReadinessFailed,
    #[error("llama.cpp sidecar startup failed: process ended before readiness")]
    EndedBeforeReady,
}

impl LlamaCppSidecarStartupError {
    #[must_use]
    pub fn memory_failure_kind(&self) -> Option<InferenceMemoryFailureKind> {
        match self {
            Self::OutOfMemory => Some(InferenceMemoryFailureKind::OutOfMemory),
            Self::ManagedBinary { .. }
            | Self::ProcessError
            | Self::ProcessTerminated { .. }
            | Self::ReadinessTimeout
            | Self::HttpReadinessFailed
            | Self::EndedBeforeReady => None,
        }
    }
}

/// Typed classification for one sidecar process event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlamaCppSidecarEventClassification {
    Output {
        line: String,
        stream: LlamaCppSidecarOutputStream,
        loggable: bool,
        listening: bool,
        failure: Option<LlamaCppSidecarStartupError>,
    },
    Failure(LlamaCppSidecarStartupError),
}

/// Sidecar process output stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaCppSidecarOutputStream {
    Stdout,
    Stderr,
}

/// Classifies bounded llama.cpp sidecar events into typed startup facts.
#[derive(Debug, Default, Clone, Copy)]
pub struct LlamaCppSidecarEventClassifier;

impl LlamaCppSidecarEventClassifier {
    #[must_use]
    pub fn classify_event(event: ProcessEvent) -> LlamaCppSidecarEventClassification {
        match event {
            ProcessEvent::Stdout(line) => {
                Self::classify_output_line(LlamaCppSidecarOutputStream::Stdout, &line)
            }
            ProcessEvent::Stderr(line) => {
                Self::classify_output_line(LlamaCppSidecarOutputStream::Stderr, &line)
            }
            ProcessEvent::Error(error) => {
                LlamaCppSidecarEventClassification::Failure(Self::classify_process_error(error))
            }
            ProcessEvent::Terminated(status) => LlamaCppSidecarEventClassification::Failure(
                LlamaCppSidecarStartupError::ProcessTerminated { status },
            ),
        }
    }

    fn classify_output_line(
        stream: LlamaCppSidecarOutputStream,
        raw_line: &[u8],
    ) -> LlamaCppSidecarEventClassification {
        let line = String::from_utf8_lossy(raw_line).to_string();
        let trimmed = line.trim();
        let loggable = is_loggable_line(trimmed);
        let listening = is_server_listening(trimmed);
        let failure = is_oom_line(trimmed).then_some(LlamaCppSidecarStartupError::OutOfMemory);
        LlamaCppSidecarEventClassification::Output {
            line,
            stream,
            loggable,
            listening,
            failure,
        }
    }

    pub fn classify_process_error(error: String) -> LlamaCppSidecarStartupError {
        if let Some(message) = crate::process::strip_managed_binary_spawn_error(&error) {
            LlamaCppSidecarStartupError::ManagedBinary { message }
        } else {
            LlamaCppSidecarStartupError::ProcessError
        }
    }
}

fn is_loggable_line(line: &str) -> bool {
    !line.contains("llama_model_loader: - kv") && !line.contains("llama_model_loader: - type")
}

fn is_server_listening(line: &str) -> bool {
    (line.contains("server") && line.contains("listening"))
        || line.contains("HTTP server listening")
}

fn is_oom_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("out of memory")
        || lower.contains("outofdevicememory")
        || lower.contains("erroroutofdevicememory")
        || lower.contains("device memory allocation")
        || (lower.contains("failed to allocate")
            && (lower.contains("vulkan") || lower.contains("cuda")))
        || (lower.contains("ggml_gallocr") && lower.contains("failed to allocate"))
        || lower.contains("graph_reserve: failed to allocate")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifier_detects_cuda_oom_without_preserving_raw_line_in_error() {
        let classification = LlamaCppSidecarEventClassifier::classify_event(ProcessEvent::Stderr(
            b"CUDA errorOutOfDeviceMemory while allocating".to_vec(),
        ));

        assert!(matches!(
            classification,
            LlamaCppSidecarEventClassification::Output {
                failure: Some(LlamaCppSidecarStartupError::OutOfMemory),
                ..
            }
        ));
    }

    #[test]
    fn classifier_detects_listening_output() {
        let classification = LlamaCppSidecarEventClassifier::classify_event(ProcessEvent::Stdout(
            b"HTTP server listening, hostname: 127.0.0.1".to_vec(),
        ));

        assert!(matches!(
            classification,
            LlamaCppSidecarEventClassification::Output {
                listening: true,
                failure: None,
                ..
            }
        ));
    }

    #[test]
    fn classifier_preserves_managed_binary_errors_as_typed_startup_failures() {
        let error = crate::process::managed_binary_spawn_error("llama.cpp is not ready");
        let classification =
            LlamaCppSidecarEventClassifier::classify_event(ProcessEvent::Error(error));

        assert_eq!(
            classification,
            LlamaCppSidecarEventClassification::Failure(
                LlamaCppSidecarStartupError::ManagedBinary {
                    message: "llama.cpp is not ready".to_string()
                }
            )
        );
    }

    #[test]
    fn oom_startup_error_maps_to_memory_failure_kind() {
        assert_eq!(
            LlamaCppSidecarStartupError::OutOfMemory.memory_failure_kind(),
            Some(InferenceMemoryFailureKind::OutOfMemory)
        );
    }
}
