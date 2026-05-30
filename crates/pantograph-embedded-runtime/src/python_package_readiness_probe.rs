//! No-shell Python package-readiness probe runner.
//!
//! This module owns the real default-host Python probing boundary for
//! package-readiness provider requests. It does not select runtimes, inspect
//! graph data, import worker modules, or run dependency-environment preflight.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use inference::{CapabilityAvailabilityId, CapabilityAvailabilityReason};
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time::timeout;

use crate::dependency_readiness::PythonPackageReadinessSnapshot;
use crate::package_readiness_provider::{
    PackageReadinessEnvironmentSelector, PackageReadinessProbeFailure,
    PackageReadinessProbeOutcome, PackageReadinessProbeRequest, PackageReadinessProbeRunner,
    PackageReadinessProviderDiagnosticCode,
};

const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PROBE_OUTPUT_BYTES: usize = 16 * 1024;
const REASON_DETAIL_MAX_CHARS: usize = 120;
const PYTHON_PACKAGE_PROBE_SCRIPT: &str = r#"
import importlib.metadata
import json
import sys

installed = []
for package_name in sys.argv[1:]:
    try:
        importlib.metadata.distribution(package_name)
        installed.append(package_name)
    except importlib.metadata.PackageNotFoundError:
        pass

print(json.dumps({"installed": installed}, separators=(",", ":")))
"#;

/// Process-backed Python package-readiness probe runner.
#[derive(Debug, Clone)]
pub struct ProcessPythonPackageReadinessProbeRunner {
    timeout: Duration,
}

impl Default for ProcessPythonPackageReadinessProbeRunner {
    fn default() -> Self {
        Self::new(DEFAULT_PROBE_TIMEOUT)
    }
}

impl ProcessPythonPackageReadinessProbeRunner {
    /// Build a process-backed probe runner with a bounded timeout.
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

#[async_trait]
impl PackageReadinessProbeRunner for ProcessPythonPackageReadinessProbeRunner {
    async fn probe(&self, request: PackageReadinessProbeRequest) -> PackageReadinessProbeOutcome {
        if request.dependency_ids.is_empty() {
            if let PackageReadinessEnvironmentSelector::PythonEnvironment { .. } =
                &request.environment
            {
                if let Err(error) = python_executable_for_probe_environment(&request.environment) {
                    return PackageReadinessProbeOutcome::Failed(vec![probe_failure_with_detail(
                        PackageReadinessProviderDiagnosticCode::PythonUnavailable,
                        None,
                        "Python runtime is not available",
                        &error,
                    )]);
                }
            }
            return PackageReadinessProbeOutcome::Snapshot(
                PythonPackageReadinessSnapshot::available(BTreeSet::new()),
            );
        }

        if let Some(failure) = first_invalid_distribution_id(&request.dependency_ids) {
            return PackageReadinessProbeOutcome::Failed(vec![failure]);
        }

        let python_executable = match python_executable_for_probe_environment(&request.environment)
        {
            Ok(path) => path,
            Err(error) => {
                return PackageReadinessProbeOutcome::Failed(vec![probe_failure_with_detail(
                    PackageReadinessProviderDiagnosticCode::PythonUnavailable,
                    None,
                    "Python runtime is not available",
                    &error,
                )]);
            }
        };

        match run_probe_process(&python_executable, &request.dependency_ids, self.timeout).await {
            Ok(installed_package_ids) => PackageReadinessProbeOutcome::Snapshot(
                PythonPackageReadinessSnapshot::available(installed_package_ids),
            ),
            Err(failure) => PackageReadinessProbeOutcome::Failed(vec![failure]),
        }
    }
}

fn python_executable_for_probe_environment(
    environment: &PackageReadinessEnvironmentSelector,
) -> Result<PathBuf, String> {
    match environment {
        PackageReadinessEnvironmentSelector::DefaultHostPython => {
            crate::python_runtime::resolve_python_executable_for_env_ids(&[])
        }
        PackageReadinessEnvironmentSelector::PythonEnvironment { environment_id } => {
            crate::python_runtime::resolve_python_executable_for_required_env_id(
                environment_id.as_str(),
            )
        }
    }
}

fn first_invalid_distribution_id(
    dependency_ids: &[CapabilityAvailabilityId],
) -> Option<PackageReadinessProbeFailure> {
    dependency_ids
        .iter()
        .find(|dependency_id| !is_safe_python_distribution_id(dependency_id.as_str()))
        .map(|dependency_id| {
            PackageReadinessProbeFailure::new(
                PackageReadinessProviderDiagnosticCode::InvalidPackageId,
                Some(dependency_id.clone()),
                reason("Package id cannot be probed as a Python distribution name."),
            )
        })
}

fn is_safe_python_distribution_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.'))
}

async fn run_probe_process(
    python_executable: &Path,
    dependency_ids: &[CapabilityAvailabilityId],
    timeout_duration: Duration,
) -> Result<BTreeSet<CapabilityAvailabilityId>, PackageReadinessProbeFailure> {
    let mut child = build_probe_command(python_executable, dependency_ids)
        .spawn()
        .map_err(|error| {
            probe_failure_with_detail(
                PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                None,
                "Failed to launch Python package-readiness probe",
                &error.to_string(),
            )
        })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        probe_failure(
            PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
            None,
            "Failed to capture Python package-readiness probe stdout.",
        )
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        probe_failure(
            PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
            None,
            "Failed to capture Python package-readiness probe stderr.",
        )
    })?;

    let run = async move {
        let (stdout, stderr, status) = tokio::try_join!(
            read_limited(stdout, "stdout"),
            read_limited(stderr, "stderr"),
            async {
                child.wait().await.map_err(|error| {
                    probe_failure_with_detail(
                        PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                        None,
                        "Failed to wait for Python package-readiness probe",
                        &error.to_string(),
                    )
                })
            }
        )?;

        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr);
            return Err(probe_failure_with_detail(
                PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                None,
                "Python package-readiness probe failed",
                stderr.trim(),
            ));
        }

        parse_probe_stdout(&stdout)
    };

    timeout(timeout_duration, run).await.unwrap_or_else(|_| {
        Err(probe_failure(
            PackageReadinessProviderDiagnosticCode::ProbeTimedOut,
            None,
            "Python package-readiness probe timed out.",
        ))
    })
}

fn build_probe_command(
    python_executable: &Path,
    dependency_ids: &[CapabilityAvailabilityId],
) -> Command {
    let mut command = Command::new(python_executable);
    command
        .arg("-I")
        .arg("-c")
        .arg(PYTHON_PACKAGE_PROBE_SCRIPT)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for dependency_id in dependency_ids {
        command.arg(dependency_id.as_str());
    }
    command
}

async fn read_limited<R>(
    reader: R,
    stream_name: &'static str,
) -> Result<Vec<u8>, PackageReadinessProbeFailure>
where
    R: AsyncRead + Unpin,
{
    let mut limited_reader = reader.take((MAX_PROBE_OUTPUT_BYTES + 1) as u64);
    let mut output = Vec::new();
    limited_reader
        .read_to_end(&mut output)
        .await
        .map_err(|error| {
            probe_failure_with_detail(
                PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                None,
                "Failed to read Python package-readiness probe output",
                &error.to_string(),
            )
        })?;
    if output.len() > MAX_PROBE_OUTPUT_BYTES {
        return Err(probe_failure_with_detail(
            PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
            None,
            "Python package-readiness probe output exceeded limit",
            stream_name,
        ));
    }
    Ok(output)
}

#[derive(Deserialize)]
struct PythonPackageProbeOutput {
    installed: Vec<String>,
}

fn parse_probe_stdout(
    stdout: &[u8],
) -> Result<BTreeSet<CapabilityAvailabilityId>, PackageReadinessProbeFailure> {
    let stdout = std::str::from_utf8(stdout).map_err(|error| {
        probe_failure_with_detail(
            PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
            None,
            "Python package-readiness probe returned non-UTF-8 stdout",
            &error.to_string(),
        )
    })?;
    let parsed =
        serde_json::from_str::<PythonPackageProbeOutput>(stdout.trim()).map_err(|error| {
            probe_failure_with_detail(
                PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                None,
                "Python package-readiness probe returned invalid JSON",
                &error.to_string(),
            )
        })?;

    parsed
        .installed
        .iter()
        .map(|dependency_id| {
            CapabilityAvailabilityId::parse(dependency_id).map_err(|error| {
                probe_failure_with_detail(
                    PackageReadinessProviderDiagnosticCode::ProbeProcessFailed,
                    None,
                    "Python package-readiness probe returned invalid package id",
                    &error.to_string(),
                )
            })
        })
        .collect()
}

fn probe_failure(
    code: PackageReadinessProviderDiagnosticCode,
    dependency_id: Option<CapabilityAvailabilityId>,
    message: &str,
) -> PackageReadinessProbeFailure {
    PackageReadinessProbeFailure::new(code, dependency_id, reason(message))
}

fn probe_failure_with_detail(
    code: PackageReadinessProviderDiagnosticCode,
    dependency_id: Option<CapabilityAvailabilityId>,
    message: &str,
    detail: &str,
) -> PackageReadinessProbeFailure {
    let detail = detail
        .chars()
        .filter(|ch| !ch.is_control())
        .take(REASON_DETAIL_MAX_CHARS)
        .collect::<String>();
    let message = if detail.is_empty() {
        message.to_string()
    } else {
        format!("{message}: {detail}")
    };
    PackageReadinessProbeFailure::new(code, dependency_id, reason(&message))
}

fn reason(message: &str) -> CapabilityAvailabilityReason {
    CapabilityAvailabilityReason::parse(message).unwrap_or_else(|_| {
        CapabilityAvailabilityReason::parse("Python package-readiness probe failed.")
            .expect("fallback reason is valid")
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn availability_id(value: &str) -> CapabilityAvailabilityId {
        CapabilityAvailabilityId::parse(value).expect("valid availability id")
    }

    fn probe_request(
        environment: PackageReadinessEnvironmentSelector,
        dependency_ids: Vec<CapabilityAvailabilityId>,
    ) -> PackageReadinessProbeRequest {
        PackageReadinessProbeRequest {
            executable_backend_key: inference::BackendId::parse("pytorch")
                .expect("valid backend id"),
            scheduler_runtime_id: availability_id("pytorch"),
            runtime_variant_id: None,
            environment,
            dependency_ids,
        }
    }

    #[tokio::test]
    async fn process_runner_requires_configured_explicit_python_environment() {
        let runner = ProcessPythonPackageReadinessProbeRunner::default();
        let outcome = runner
            .probe(probe_request(
                PackageReadinessEnvironmentSelector::PythonEnvironment {
                    environment_id: availability_id("pantograph-test-missing-env"),
                },
                vec![availability_id("diffusers")],
            ))
            .await;

        let PackageReadinessProbeOutcome::Failed(failures) = outcome else {
            panic!("expected explicit environment failure");
        };
        assert_eq!(
            failures[0].code,
            PackageReadinessProviderDiagnosticCode::PythonUnavailable
        );
        assert!(failures[0]
            .reason
            .as_str()
            .contains("pantograph-test-missing-env"));
    }

    #[tokio::test]
    async fn process_runner_rejects_unprobeable_package_id_without_process() {
        let runner = ProcessPythonPackageReadinessProbeRunner::default();
        let outcome = runner
            .probe(probe_request(
                PackageReadinessEnvironmentSelector::DefaultHostPython,
                vec![availability_id("pytorch:cuda")],
            ))
            .await;

        let PackageReadinessProbeOutcome::Failed(failures) = outcome else {
            panic!("expected invalid package id failure");
        };
        assert_eq!(
            failures[0].code,
            PackageReadinessProviderDiagnosticCode::InvalidPackageId
        );
        assert_eq!(
            failures[0].dependency_id.as_ref().map(|id| id.as_str()),
            Some("pytorch:cuda")
        );
    }

    #[test]
    fn parse_probe_stdout_returns_validated_installed_package_ids() {
        let installed = parse_probe_stdout(br#"{"installed":["diffusers","torch"]}"#)
            .expect("parse installed package ids");

        assert_eq!(
            installed
                .iter()
                .map(CapabilityAvailabilityId::as_str)
                .collect::<Vec<_>>(),
            vec!["diffusers", "torch"]
        );
    }

    #[test]
    fn parse_probe_stdout_rejects_invalid_json() {
        let failure = parse_probe_stdout(b"not-json").expect_err("invalid json fails");

        assert_eq!(
            failure.code,
            PackageReadinessProviderDiagnosticCode::ProbeProcessFailed
        );
    }

    #[test]
    fn probe_command_uses_no_shell_script_and_package_args() {
        let command = build_probe_command(
            &PathBuf::from("/usr/bin/python3"),
            &[availability_id("diffusers"), availability_id("torch")],
        );
        let args = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(args[0], "-I");
        assert_eq!(args[1], "-c");
        assert_eq!(args[3], "diffusers");
        assert_eq!(args[4], "torch");
    }
}
