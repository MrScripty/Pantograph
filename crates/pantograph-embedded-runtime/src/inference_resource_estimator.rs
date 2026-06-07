use inference::{
    InferenceResourceEstimate, InferenceResourceEstimateDiagnostic,
    InferenceResourceEstimateDiagnosticCode, InferenceResourceEstimateKind,
    InferenceUnavailableResourceEstimateState, PackageFactStatus, PackageFactValueSource,
    PackageLogicalSizeFacts,
};
use pantograph_scheduler::{SchedulerEstimateHint, SchedulerEstimateHintKind};

const SCHEDULER_MAX_ESTIMATE_VALUE: u64 = i64::MAX as u64;
const STATIC_LOAD_MEMORY_MULTIPLIER: u64 = 3;
const STATIC_RUNTIME_OVERHEAD_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConservativeInferenceResourceEstimates {
    pub estimates: Vec<InferenceResourceEstimate>,
    pub scheduler_hints: Vec<SchedulerEstimateHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConservativeMemoryEstimate {
    model_residency_bytes: u64,
    runtime_overhead_bytes: u64,
    peak_memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConservativeMemoryEstimateError {
    InsufficientFacts(InferenceResourceEstimateDiagnostic),
    Overflow(InferenceResourceEstimateDiagnostic),
}

pub(crate) fn conservative_estimates_from_package_logical_size(
    logical_size: &PackageLogicalSizeFacts,
) -> ConservativeInferenceResourceEstimates {
    match conservative_memory_estimate(logical_size) {
        Ok(estimate) => available_estimates(estimate),
        Err(error) => unavailable_estimates(error),
    }
}

pub(crate) fn conservative_loaded_runtime_memory_estimate_bytes(
    logical_size: &PackageLogicalSizeFacts,
) -> Option<u64> {
    conservative_memory_estimate(logical_size)
        .ok()
        .map(|estimate| estimate.model_residency_bytes)
}

fn conservative_memory_estimate(
    logical_size: &PackageLogicalSizeFacts,
) -> Result<ConservativeMemoryEstimate, ConservativeMemoryEstimateError> {
    let logical_bytes = logical_size_bytes(logical_size)?;
    let model_residency_bytes = logical_bytes
        .checked_mul(STATIC_LOAD_MEMORY_MULTIPLIER)
        .ok_or_else(|| {
            ConservativeMemoryEstimateError::Overflow(diagnostic(
                InferenceResourceEstimateDiagnosticCode::ArithmeticOverflow,
                "artifact.logical_size.total_size_bytes",
                "conservative loaded-memory estimate overflowed byte arithmetic",
            ))
        })?;
    let peak_memory_bytes = model_residency_bytes
        .checked_add(STATIC_RUNTIME_OVERHEAD_BYTES)
        .ok_or_else(|| {
            ConservativeMemoryEstimateError::Overflow(diagnostic(
                InferenceResourceEstimateDiagnosticCode::ArithmeticOverflow,
                "artifact.logical_size.total_size_bytes",
                "conservative peak-memory estimate overflowed byte arithmetic",
            ))
        })?;
    if peak_memory_bytes > SCHEDULER_MAX_ESTIMATE_VALUE {
        return Err(ConservativeMemoryEstimateError::Overflow(diagnostic(
            InferenceResourceEstimateDiagnosticCode::ArithmeticOverflow,
            "artifact.logical_size.total_size_bytes",
            "conservative peak-memory estimate exceeds scheduler resource arithmetic bounds",
        )));
    }

    Ok(ConservativeMemoryEstimate {
        model_residency_bytes,
        runtime_overhead_bytes: STATIC_RUNTIME_OVERHEAD_BYTES,
        peak_memory_bytes,
    })
}

fn logical_size_bytes(
    logical_size: &PackageLogicalSizeFacts,
) -> Result<u64, ConservativeMemoryEstimateError> {
    if !is_usable_size_source(logical_size.value_source) {
        return Err(ConservativeMemoryEstimateError::InsufficientFacts(
            diagnostic(
                InferenceResourceEstimateDiagnosticCode::InsufficientFacts,
                "artifact.logical_size.value_source",
                "logical package size source is not strong enough for conservative resource estimation",
            ),
        ));
    }

    if let Some(total_size_bytes) = logical_size.total_size_bytes {
        if total_size_bytes == 0 {
            return Err(ConservativeMemoryEstimateError::InsufficientFacts(
                diagnostic(
                    InferenceResourceEstimateDiagnosticCode::InvalidInput,
                    "artifact.logical_size.total_size_bytes",
                    "logical package size must be greater than zero",
                ),
            ));
        }
        return Ok(total_size_bytes);
    }

    let mut total_size_bytes = 0_u64;
    let mut usable_file_count = 0_usize;
    for file in &logical_size.files {
        if file.status != PackageFactStatus::Present || !is_usable_size_source(file.value_source) {
            continue;
        }
        let Some(size_bytes) = file.size_bytes else {
            continue;
        };
        if size_bytes == 0 {
            continue;
        }
        total_size_bytes = total_size_bytes.checked_add(size_bytes).ok_or_else(|| {
            ConservativeMemoryEstimateError::Overflow(diagnostic(
                InferenceResourceEstimateDiagnosticCode::ArithmeticOverflow,
                "artifact.logical_size.files",
                "logical package file sizes overflowed byte arithmetic",
            ))
        })?;
        usable_file_count += 1;
    }

    if usable_file_count == 0 {
        return Err(ConservativeMemoryEstimateError::InsufficientFacts(
            diagnostic(
                InferenceResourceEstimateDiagnosticCode::InsufficientFacts,
                "artifact.logical_size",
                "logical package size facts must include a total size or at least one present file size",
            ),
        ));
    }

    Ok(total_size_bytes)
}

fn is_usable_size_source(source: PackageFactValueSource) -> bool {
    matches!(
        source,
        PackageFactValueSource::Header
            | PackageFactValueSource::Config
            | PackageFactValueSource::FilesystemMetadata
            | PackageFactValueSource::UpstreamMetadata
            | PackageFactValueSource::ComponentLayout
    )
}

fn available_estimates(
    estimate: ConservativeMemoryEstimate,
) -> ConservativeInferenceResourceEstimates {
    ConservativeInferenceResourceEstimates {
        estimates: vec![
            InferenceResourceEstimate::available(
                InferenceResourceEstimateKind::ModelResidencyBytes,
                estimate.model_residency_bytes,
            ),
            InferenceResourceEstimate::available(
                InferenceResourceEstimateKind::RuntimeOverheadBytes,
                estimate.runtime_overhead_bytes,
            ),
            InferenceResourceEstimate::available(
                InferenceResourceEstimateKind::PeakRamBytes,
                estimate.peak_memory_bytes,
            ),
            InferenceResourceEstimate::available(
                InferenceResourceEstimateKind::PeakVramBytes,
                estimate.peak_memory_bytes,
            ),
        ],
        scheduler_hints: vec![
            SchedulerEstimateHint {
                kind: SchedulerEstimateHintKind::PeakRamBytes,
                value: estimate.peak_memory_bytes,
            },
            SchedulerEstimateHint {
                kind: SchedulerEstimateHintKind::PeakVramBytes,
                value: estimate.peak_memory_bytes,
            },
        ],
    }
}

fn unavailable_estimates(
    error: ConservativeMemoryEstimateError,
) -> ConservativeInferenceResourceEstimates {
    let (state, diagnostic) = match error {
        ConservativeMemoryEstimateError::InsufficientFacts(diagnostic) => (
            InferenceUnavailableResourceEstimateState::InsufficientFacts,
            diagnostic,
        ),
        ConservativeMemoryEstimateError::Overflow(diagnostic) => (
            InferenceUnavailableResourceEstimateState::Overflow,
            diagnostic,
        ),
    };
    let estimates = [
        InferenceResourceEstimateKind::ModelResidencyBytes,
        InferenceResourceEstimateKind::RuntimeOverheadBytes,
        InferenceResourceEstimateKind::PeakRamBytes,
        InferenceResourceEstimateKind::PeakVramBytes,
    ]
    .into_iter()
    .map(|kind| InferenceResourceEstimate::unavailable(kind, state, vec![diagnostic.clone()]))
    .collect();

    ConservativeInferenceResourceEstimates {
        estimates,
        scheduler_hints: Vec::new(),
    }
}

fn diagnostic(
    code: InferenceResourceEstimateDiagnosticCode,
    field_path: &'static str,
    message: &'static str,
) -> InferenceResourceEstimateDiagnostic {
    InferenceResourceEstimateDiagnostic::error(code, field_path, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inference::{ModelPackageDiagnostic, PackageFileSizeFact, PackageSizeRole};

    #[test]
    fn estimates_peak_scheduler_hints_from_package_total_size() {
        let estimates =
            conservative_estimates_from_package_logical_size(&logical_size_with_total(1_024));

        assert_eq!(
            estimates.scheduler_hints,
            vec![
                SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakRamBytes,
                    value: STATIC_RUNTIME_OVERHEAD_BYTES + 3_072,
                },
                SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakVramBytes,
                    value: STATIC_RUNTIME_OVERHEAD_BYTES + 3_072,
                },
            ]
        );
        assert_eq!(
            estimate_value(
                &estimates.estimates,
                InferenceResourceEstimateKind::ModelResidencyBytes
            ),
            Some(3_072)
        );
        assert_eq!(
            estimate_value(
                &estimates.estimates,
                InferenceResourceEstimateKind::RuntimeOverheadBytes
            ),
            Some(STATIC_RUNTIME_OVERHEAD_BYTES)
        );
    }

    #[test]
    fn loaded_runtime_memory_estimate_uses_model_residency_component() {
        let estimate =
            conservative_loaded_runtime_memory_estimate_bytes(&logical_size_with_total(1_024));

        assert_eq!(estimate, Some(3_072));
    }

    #[test]
    fn loaded_runtime_memory_estimate_is_absent_for_insufficient_size_facts() {
        let estimate =
            conservative_loaded_runtime_memory_estimate_bytes(&PackageLogicalSizeFacts {
                total_size_bytes: None,
                value_source: PackageFactValueSource::FilesystemMetadata,
                files: Vec::new(),
                diagnostics: Vec::new(),
            });

        assert_eq!(estimate, None);
    }

    #[test]
    fn sums_present_file_sizes_when_total_size_is_absent() {
        let estimates =
            conservative_estimates_from_package_logical_size(&logical_size_with_files(vec![
                file_size(
                    "model-00001.safetensors",
                    Some(2_048),
                    PackageFactStatus::Present,
                ),
                file_size(
                    "model-00002.safetensors",
                    Some(4_096),
                    PackageFactStatus::Present,
                ),
                file_size(
                    "missing.safetensors",
                    Some(1_024),
                    PackageFactStatus::Missing,
                ),
            ]));

        assert_eq!(
            estimate_value(
                &estimates.estimates,
                InferenceResourceEstimateKind::PeakRamBytes
            ),
            Some(STATIC_RUNTIME_OVERHEAD_BYTES + 18_432)
        );
    }

    #[test]
    fn rejects_missing_size_facts_with_typed_unavailable_estimates() {
        let estimates =
            conservative_estimates_from_package_logical_size(&PackageLogicalSizeFacts {
                total_size_bytes: None,
                value_source: PackageFactValueSource::FilesystemMetadata,
                files: Vec::new(),
                diagnostics: Vec::new(),
            });

        assert!(estimates.scheduler_hints.is_empty());
        assert!(estimates.estimates.iter().all(|estimate| {
            estimate.state() == inference::InferenceResourceEstimateState::InsufficientFacts
                && estimate.value_bytes().is_none()
                && estimate.diagnostics()[0].code
                    == InferenceResourceEstimateDiagnosticCode::InsufficientFacts
        }));
    }

    #[test]
    fn rejects_weak_size_source_with_typed_unavailable_estimates() {
        let estimates =
            conservative_estimates_from_package_logical_size(&PackageLogicalSizeFacts {
                total_size_bytes: Some(1_024),
                value_source: PackageFactValueSource::FilenameWeak,
                files: Vec::new(),
                diagnostics: Vec::new(),
            });

        assert!(estimates.scheduler_hints.is_empty());
        assert_eq!(
            estimates.estimates[0].state(),
            inference::InferenceResourceEstimateState::InsufficientFacts
        );
    }

    #[test]
    fn rejects_scheduler_bound_overflow_before_publishing_hints() {
        let too_large = (SCHEDULER_MAX_ESTIMATE_VALUE / STATIC_LOAD_MEMORY_MULTIPLIER)
            + STATIC_RUNTIME_OVERHEAD_BYTES;
        let estimates =
            conservative_estimates_from_package_logical_size(&logical_size_with_total(too_large));

        assert!(estimates.scheduler_hints.is_empty());
        assert!(estimates.estimates.iter().all(|estimate| {
            estimate.state() == inference::InferenceResourceEstimateState::Overflow
                && estimate.value_bytes().is_none()
        }));
    }

    fn logical_size_with_total(total_size_bytes: u64) -> PackageLogicalSizeFacts {
        PackageLogicalSizeFacts {
            total_size_bytes: Some(total_size_bytes),
            value_source: PackageFactValueSource::FilesystemMetadata,
            files: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn logical_size_with_files(files: Vec<PackageFileSizeFact>) -> PackageLogicalSizeFacts {
        PackageLogicalSizeFacts {
            total_size_bytes: None,
            value_source: PackageFactValueSource::FilesystemMetadata,
            files,
            diagnostics: vec![ModelPackageDiagnostic {
                code: "total_size_unavailable".to_string(),
                message: "total package size is unavailable".to_string(),
                path: None,
            }],
        }
    }

    fn file_size(
        relative_path: &str,
        size_bytes: Option<u64>,
        status: PackageFactStatus,
    ) -> PackageFileSizeFact {
        PackageFileSizeFact {
            relative_path: relative_path.to_string(),
            size_bytes,
            status,
            value_source: PackageFactValueSource::FilesystemMetadata,
            role: Some(PackageSizeRole::Shard),
        }
    }

    fn estimate_value(
        estimates: &[InferenceResourceEstimate],
        kind: InferenceResourceEstimateKind,
    ) -> Option<u64> {
        estimates
            .iter()
            .find(|estimate| estimate.kind() == kind)
            .and_then(InferenceResourceEstimate::value_bytes)
    }
}
