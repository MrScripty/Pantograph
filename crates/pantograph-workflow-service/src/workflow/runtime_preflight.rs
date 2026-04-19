use pantograph_runtime_identity::canonical_runtime_backend_key;

use super::{
    WorkflowIoResponse, WorkflowRuntimeCapability, WorkflowRuntimeInstallState,
    WorkflowRuntimeIssue, WorkflowRuntimeReadinessState,
};

pub(super) fn collect_preflight_warnings(
    io: &WorkflowIoResponse,
    runtime_warnings: &[WorkflowRuntimeIssue],
    blocking_runtime_issues: &[WorkflowRuntimeIssue],
) -> Vec<String> {
    let mut warnings = io
        .inputs
        .iter()
        .flat_map(|node| {
            node.ports.iter().filter_map(move |port| {
                if port.required.is_none() {
                    Some(format!(
                        "input surface '{}.{}' does not declare required metadata; preflight treats it as optional",
                        node.node_id, port.port_id
                    ))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();
    warnings.sort();
    warnings.extend(runtime_warnings.iter().map(|issue| issue.message.clone()));
    warnings.extend(
        blocking_runtime_issues
            .iter()
            .map(|issue| issue.message.clone()),
    );
    warnings.sort();
    warnings.dedup();
    warnings
}

pub fn evaluate_runtime_preflight(
    required_backends: &[String],
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> (Vec<WorkflowRuntimeIssue>, Vec<WorkflowRuntimeIssue>) {
    let mut runtime_warnings = Vec::new();
    let mut blocking_runtime_issues = Vec::new();

    for required_backend_key in required_backends {
        let required_backend_key = required_backend_key.trim();
        if required_backend_key.is_empty() {
            continue;
        }

        let Some(runtime) = find_runtime_capability(required_backend_key, runtime_capabilities)
        else {
            let issue = WorkflowRuntimeIssue {
                runtime_id: required_backend_key.to_string(),
                display_name: required_backend_key.to_string(),
                required_backend_key: required_backend_key.to_string(),
                message: format!(
                    "workflow requires backend '{}' but no matching runtime capability is available",
                    required_backend_key
                ),
            };
            runtime_warnings.push(issue.clone());
            blocking_runtime_issues.push(issue);
            continue;
        };

        if runtime.available && runtime.configured {
            continue;
        }

        let issue = runtime_issue_for_capability(runtime, required_backend_key);
        runtime_warnings.push(issue.clone());
        blocking_runtime_issues.push(issue);
    }

    dedup_runtime_issues(&mut runtime_warnings);
    dedup_runtime_issues(&mut blocking_runtime_issues);
    (runtime_warnings, blocking_runtime_issues)
}

pub fn format_runtime_not_ready_message(issues: &[WorkflowRuntimeIssue]) -> String {
    issues
        .iter()
        .map(|issue| issue.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn runtime_issue_for_capability(
    runtime: &WorkflowRuntimeCapability,
    required_backend_key: &str,
) -> WorkflowRuntimeIssue {
    WorkflowRuntimeIssue {
        runtime_id: runtime.runtime_id.clone(),
        display_name: runtime.display_name.clone(),
        required_backend_key: required_backend_key.to_string(),
        message: describe_runtime_issue(runtime, required_backend_key),
    }
}

fn dedup_runtime_issues(issues: &mut Vec<WorkflowRuntimeIssue>) {
    issues.sort_by(|a, b| {
        a.runtime_id
            .cmp(&b.runtime_id)
            .then_with(|| a.required_backend_key.cmp(&b.required_backend_key))
    });
    issues.dedup_by(|left, right| {
        left.runtime_id == right.runtime_id
            && left.required_backend_key == right.required_backend_key
            && left.message == right.message
    });
}

fn find_runtime_capability<'a>(
    required_backend_key: &str,
    runtime_capabilities: &'a [WorkflowRuntimeCapability],
) -> Option<&'a WorkflowRuntimeCapability> {
    let normalized_required_backend_key = canonical_runtime_backend_key(required_backend_key);
    runtime_capabilities
        .iter()
        .filter(|runtime| {
            runtime_matches_required_backend(runtime, &normalized_required_backend_key)
        })
        .max_by(|left, right| {
            runtime_capability_match_rank(left)
                .cmp(&runtime_capability_match_rank(right))
                .then_with(|| left.runtime_id.cmp(&right.runtime_id))
        })
}

fn runtime_matches_required_backend(
    runtime: &WorkflowRuntimeCapability,
    normalized_required_backend_key: &str,
) -> bool {
    canonical_runtime_backend_key(&runtime.runtime_id) == normalized_required_backend_key
        || runtime.backend_keys.iter().any(|backend_key| {
            canonical_runtime_backend_key(backend_key) == normalized_required_backend_key
        })
}

fn runtime_capability_match_rank(
    runtime: &WorkflowRuntimeCapability,
) -> (bool, bool, bool, bool, u8) {
    (
        runtime.selected,
        runtime.available && runtime.configured,
        runtime.configured,
        runtime.available,
        runtime_install_state_rank(runtime.install_state),
    )
}

fn runtime_install_state_rank(install_state: WorkflowRuntimeInstallState) -> u8 {
    match install_state {
        WorkflowRuntimeInstallState::Installed | WorkflowRuntimeInstallState::SystemProvided => 3,
        WorkflowRuntimeInstallState::Missing => 2,
        WorkflowRuntimeInstallState::Unsupported => 1,
    }
}

fn describe_runtime_issue(
    runtime: &WorkflowRuntimeCapability,
    required_backend_key: &str,
) -> String {
    if !runtime.configured {
        if let Some(reason) = runtime
            .unavailable_reason
            .as_ref()
            .filter(|reason| !reason.trim().is_empty())
        {
            return format_runtime_issue_message(runtime, required_backend_key, reason);
        }

        if let Some(readiness_state) = runtime.readiness_state {
            let readiness_detail = runtime
                .selected_version
                .as_ref()
                .map(|version| {
                    format!(
                        "{} selected version '{}' is {}",
                        runtime.display_name,
                        version,
                        runtime_readiness_label(readiness_state)
                    )
                })
                .unwrap_or_else(|| {
                    format!(
                        "{} is {}",
                        runtime.display_name,
                        runtime_readiness_label(readiness_state)
                    )
                });
            return format_runtime_issue_message(runtime, required_backend_key, &readiness_detail);
        }

        return format!(
            "workflow requires backend '{}' but {} is not configured",
            required_backend_key, runtime.display_name
        );
    }

    match runtime.install_state {
        WorkflowRuntimeInstallState::Missing => {
            format!(
                "workflow requires backend '{}' but {} is not installed",
                required_backend_key, runtime.display_name
            )
        }
        WorkflowRuntimeInstallState::Unsupported => format!(
            "workflow requires backend '{}' but {} is unsupported on this platform",
            required_backend_key, runtime.display_name
        ),
        WorkflowRuntimeInstallState::Installed | WorkflowRuntimeInstallState::SystemProvided => {
            runtime.unavailable_reason.clone().unwrap_or_else(|| {
                format!(
                    "workflow requires backend '{}' but {} is not ready",
                    required_backend_key, runtime.display_name
                )
            })
        }
    }
}

fn format_runtime_issue_message(
    runtime: &WorkflowRuntimeCapability,
    required_backend_key: &str,
    detail: &str,
) -> String {
    let trimmed_detail = detail.trim();
    if trimmed_detail.is_empty() {
        return format!(
            "workflow requires backend '{}' but {} is not ready",
            required_backend_key, runtime.display_name
        );
    }

    if trimmed_detail.contains(&runtime.display_name) {
        return format!(
            "workflow requires backend '{}' but {}",
            required_backend_key, trimmed_detail
        );
    }

    format!(
        "workflow requires backend '{}' but {}: {}",
        required_backend_key, runtime.display_name, trimmed_detail
    )
}

fn runtime_readiness_label(state: WorkflowRuntimeReadinessState) -> &'static str {
    match state {
        WorkflowRuntimeReadinessState::Unknown => "unknown",
        WorkflowRuntimeReadinessState::Missing => "missing",
        WorkflowRuntimeReadinessState::Downloading => "downloading",
        WorkflowRuntimeReadinessState::Extracting => "extracting",
        WorkflowRuntimeReadinessState::Validating => "validating",
        WorkflowRuntimeReadinessState::Ready => "ready",
        WorkflowRuntimeReadinessState::Failed => "failed",
        WorkflowRuntimeReadinessState::Unsupported => "unsupported on this platform",
    }
}
