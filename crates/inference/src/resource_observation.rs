//! Typed execution resource observations emitted by inference runtimes.
//!
//! These contracts describe facts observed during execution. They do not own
//! scheduler policy, terminal-event persistence, or platform-specific probing.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const INFERENCE_RESOURCE_OBSERVATION_SOURCE_LIMIT: usize = 8;
pub const INFERENCE_RESOURCE_OBSERVATION_AVAILABILITY_LIMIT: usize = 16;

/// Resource metric represented by one observation fact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InferenceResourceObservationMetricKind {
    PeakRamBytes,
    PeakVramBytes,
}

/// Backend/runtime source that produced or attempted to produce one metric.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InferenceResourceObservationSourceKind {
    PytorchCuda,
    PytorchMps,
    PytorchCpu,
    OsProcessRss,
    ManagedRuntimeTelemetry,
    ExternalRuntimeAdapter,
}

/// Explicit state for a metric a producer knows about but cannot report.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InferenceResourceObservationUnavailableState {
    NotAvailable,
    NotImplemented,
    RuntimeNotInstalled,
    UnsupportedDevice,
    UnsupportedPlatform,
    MissingRuntimeCapability,
}

/// Typed memory-failure facts observed by a runtime/backend adapter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InferenceMemoryFailureKind {
    OutOfMemory,
}

/// Source attribution for a metric value or availability fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub struct InferenceResourceObservationSource {
    metric_kind: InferenceResourceObservationMetricKind,
    source_kind: InferenceResourceObservationSourceKind,
}

impl InferenceResourceObservationSource {
    #[must_use]
    pub fn new(
        metric_kind: InferenceResourceObservationMetricKind,
        source_kind: InferenceResourceObservationSourceKind,
    ) -> Self {
        Self {
            metric_kind,
            source_kind,
        }
    }

    #[must_use]
    pub fn metric_kind(&self) -> InferenceResourceObservationMetricKind {
        self.metric_kind
    }

    #[must_use]
    pub fn source_kind(&self) -> InferenceResourceObservationSourceKind {
        self.source_kind
    }
}

/// Availability fact for a metric that cannot be produced on this execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub struct InferenceResourceObservationAvailability {
    metric_kind: InferenceResourceObservationMetricKind,
    state: InferenceResourceObservationUnavailableState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_kind: Option<InferenceResourceObservationSourceKind>,
}

impl InferenceResourceObservationAvailability {
    #[must_use]
    pub fn new(
        metric_kind: InferenceResourceObservationMetricKind,
        state: InferenceResourceObservationUnavailableState,
        source_kind: Option<InferenceResourceObservationSourceKind>,
    ) -> Self {
        Self {
            metric_kind,
            state,
            source_kind,
        }
    }

    #[must_use]
    pub fn metric_kind(&self) -> InferenceResourceObservationMetricKind {
        self.metric_kind
    }

    #[must_use]
    pub fn state(&self) -> InferenceResourceObservationUnavailableState {
        self.state
    }

    #[must_use]
    pub fn source_kind(&self) -> Option<InferenceResourceObservationSourceKind> {
        self.source_kind
    }
}

/// Validated execution resource observation emitted by an inference producer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "snake_case",
    try_from = "InferenceExecutionResourceObservationWire",
    into = "InferenceExecutionResourceObservationWire"
)]
#[must_use]
pub struct InferenceExecutionResourceObservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peak_ram_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peak_vram_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memory_failure_kind: Option<InferenceMemoryFailureKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sources: Vec<InferenceResourceObservationSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    availability: Vec<InferenceResourceObservationAvailability>,
}

impl InferenceExecutionResourceObservation {
    pub fn new(
        peak_ram_bytes: Option<u64>,
        peak_vram_bytes: Option<u64>,
        memory_failure_kind: Option<InferenceMemoryFailureKind>,
        sources: Vec<InferenceResourceObservationSource>,
        availability: Vec<InferenceResourceObservationAvailability>,
    ) -> Result<Self, InferenceExecutionResourceObservationError> {
        validate_resource_observation_parts(
            peak_ram_bytes,
            peak_vram_bytes,
            memory_failure_kind,
            sources,
            availability,
        )
    }

    pub fn peak_ram(
        value_bytes: u64,
        source_kind: InferenceResourceObservationSourceKind,
    ) -> Result<Self, InferenceExecutionResourceObservationError> {
        Self::new(
            Some(value_bytes),
            None,
            None,
            vec![InferenceResourceObservationSource::new(
                InferenceResourceObservationMetricKind::PeakRamBytes,
                source_kind,
            )],
            Vec::new(),
        )
    }

    pub fn peak_vram(
        value_bytes: u64,
        source_kind: InferenceResourceObservationSourceKind,
    ) -> Result<Self, InferenceExecutionResourceObservationError> {
        Self::new(
            None,
            Some(value_bytes),
            None,
            vec![InferenceResourceObservationSource::new(
                InferenceResourceObservationMetricKind::PeakVramBytes,
                source_kind,
            )],
            Vec::new(),
        )
    }

    pub fn unavailable(
        availability: Vec<InferenceResourceObservationAvailability>,
    ) -> Result<Self, InferenceExecutionResourceObservationError> {
        Self::new(None, None, None, Vec::new(), availability)
    }

    pub fn memory_failure(memory_failure_kind: InferenceMemoryFailureKind) -> Self {
        Self {
            peak_ram_bytes: None,
            peak_vram_bytes: None,
            memory_failure_kind: Some(memory_failure_kind),
            sources: Vec::new(),
            availability: Vec::new(),
        }
    }

    pub fn merge(self, other: Self) -> Result<Self, InferenceExecutionResourceObservationError> {
        let memory_failure_kind = self.memory_failure_kind.or(other.memory_failure_kind);
        let mut sources = self.sources;
        sources.extend(other.sources);
        let mut availability = self.availability;
        availability.extend(other.availability);

        Self::new(
            max_optional_u64(self.peak_ram_bytes, other.peak_ram_bytes),
            max_optional_u64(self.peak_vram_bytes, other.peak_vram_bytes),
            memory_failure_kind,
            sources,
            availability,
        )
    }

    #[must_use]
    pub fn peak_ram_bytes(&self) -> Option<u64> {
        self.peak_ram_bytes
    }

    #[must_use]
    pub fn peak_vram_bytes(&self) -> Option<u64> {
        self.peak_vram_bytes
    }

    #[must_use]
    pub fn memory_failure_kind(&self) -> Option<InferenceMemoryFailureKind> {
        self.memory_failure_kind
    }

    #[must_use]
    pub fn sources(&self) -> &[InferenceResourceObservationSource] {
        &self.sources
    }

    #[must_use]
    pub fn availability(&self) -> &[InferenceResourceObservationAvailability] {
        &self.availability
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct InferenceExecutionResourceObservationWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peak_ram_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peak_vram_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    memory_failure_kind: Option<InferenceMemoryFailureKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sources: Vec<InferenceResourceObservationSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    availability: Vec<InferenceResourceObservationAvailability>,
}

impl From<InferenceExecutionResourceObservation> for InferenceExecutionResourceObservationWire {
    fn from(observation: InferenceExecutionResourceObservation) -> Self {
        Self {
            peak_ram_bytes: observation.peak_ram_bytes,
            peak_vram_bytes: observation.peak_vram_bytes,
            memory_failure_kind: observation.memory_failure_kind,
            sources: observation.sources,
            availability: observation.availability,
        }
    }
}

impl TryFrom<InferenceExecutionResourceObservationWire> for InferenceExecutionResourceObservation {
    type Error = InferenceExecutionResourceObservationError;

    fn try_from(wire: InferenceExecutionResourceObservationWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.peak_ram_bytes,
            wire.peak_vram_bytes,
            wire.memory_failure_kind,
            wire.sources,
            wire.availability,
        )
    }
}

/// Error returned when constructing an invalid resource observation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceExecutionResourceObservationError {
    #[error("resource observation must contain at least one metric, memory failure, or availability fact")]
    EmptyObservation,
    #[error("{metric_kind:?} must be greater than zero when reported as available")]
    ZeroPeakValue {
        metric_kind: InferenceResourceObservationMetricKind,
    },
    #[error("resource observation carries {actual_len} sources, exceeding the limit of {max_len}")]
    TooManySources { max_len: usize, actual_len: usize },
    #[error("resource observation carries {actual_len} availability facts, exceeding the limit of {max_len}")]
    TooManyAvailabilityFacts { max_len: usize, actual_len: usize },
    #[error("source for {metric_kind:?} has no matching metric or availability fact")]
    SourceWithoutMetricFact {
        metric_kind: InferenceResourceObservationMetricKind,
    },
}

fn validate_resource_observation_parts(
    peak_ram_bytes: Option<u64>,
    peak_vram_bytes: Option<u64>,
    memory_failure_kind: Option<InferenceMemoryFailureKind>,
    sources: Vec<InferenceResourceObservationSource>,
    availability: Vec<InferenceResourceObservationAvailability>,
) -> Result<InferenceExecutionResourceObservation, InferenceExecutionResourceObservationError> {
    if matches!(peak_ram_bytes, Some(0)) {
        return Err(InferenceExecutionResourceObservationError::ZeroPeakValue {
            metric_kind: InferenceResourceObservationMetricKind::PeakRamBytes,
        });
    }
    if matches!(peak_vram_bytes, Some(0)) {
        return Err(InferenceExecutionResourceObservationError::ZeroPeakValue {
            metric_kind: InferenceResourceObservationMetricKind::PeakVramBytes,
        });
    }
    if sources.len() > INFERENCE_RESOURCE_OBSERVATION_SOURCE_LIMIT {
        return Err(InferenceExecutionResourceObservationError::TooManySources {
            max_len: INFERENCE_RESOURCE_OBSERVATION_SOURCE_LIMIT,
            actual_len: sources.len(),
        });
    }
    if availability.len() > INFERENCE_RESOURCE_OBSERVATION_AVAILABILITY_LIMIT {
        return Err(
            InferenceExecutionResourceObservationError::TooManyAvailabilityFacts {
                max_len: INFERENCE_RESOURCE_OBSERVATION_AVAILABILITY_LIMIT,
                actual_len: availability.len(),
            },
        );
    }

    let sources = normalize_ordered(sources);
    let availability = normalize_ordered(availability);

    if peak_ram_bytes.is_none()
        && peak_vram_bytes.is_none()
        && memory_failure_kind.is_none()
        && availability.is_empty()
    {
        return Err(InferenceExecutionResourceObservationError::EmptyObservation);
    }

    let availability_metrics: BTreeSet<_> = availability
        .iter()
        .map(InferenceResourceObservationAvailability::metric_kind)
        .collect();
    for source in &sources {
        if !metric_has_fact(
            source.metric_kind(),
            peak_ram_bytes,
            peak_vram_bytes,
            &availability_metrics,
        ) {
            return Err(
                InferenceExecutionResourceObservationError::SourceWithoutMetricFact {
                    metric_kind: source.metric_kind(),
                },
            );
        }
    }

    Ok(InferenceExecutionResourceObservation {
        peak_ram_bytes,
        peak_vram_bytes,
        memory_failure_kind,
        sources,
        availability,
    })
}

fn metric_has_fact(
    metric_kind: InferenceResourceObservationMetricKind,
    peak_ram_bytes: Option<u64>,
    peak_vram_bytes: Option<u64>,
    availability_metrics: &BTreeSet<InferenceResourceObservationMetricKind>,
) -> bool {
    (match metric_kind {
        InferenceResourceObservationMetricKind::PeakRamBytes => peak_ram_bytes.is_some(),
        InferenceResourceObservationMetricKind::PeakVramBytes => peak_vram_bytes.is_some(),
    }) || availability_metrics.contains(&metric_kind)
}

fn normalize_ordered<T>(values: Vec<T>) -> Vec<T>
where
    T: Ord,
{
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn max_optional_u64(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        metric_kind: InferenceResourceObservationMetricKind,
        source_kind: InferenceResourceObservationSourceKind,
    ) -> InferenceResourceObservationSource {
        InferenceResourceObservationSource::new(metric_kind, source_kind)
    }

    fn unavailable(
        metric_kind: InferenceResourceObservationMetricKind,
        state: InferenceResourceObservationUnavailableState,
        source_kind: InferenceResourceObservationSourceKind,
    ) -> InferenceResourceObservationAvailability {
        InferenceResourceObservationAvailability::new(metric_kind, state, Some(source_kind))
    }

    #[test]
    fn available_resource_observation_round_trips_with_typed_sources() {
        let observation = InferenceExecutionResourceObservation::new(
            Some(4096),
            Some(2048),
            None,
            vec![
                source(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                ),
                source(
                    InferenceResourceObservationMetricKind::PeakVramBytes,
                    InferenceResourceObservationSourceKind::PytorchCuda,
                ),
            ],
            Vec::new(),
        )
        .expect("valid observation");

        let encoded = serde_json::to_value(&observation).expect("observation encodes");
        assert_eq!(
            encoded,
            serde_json::json!({
                "peak_ram_bytes": 4096,
                "peak_vram_bytes": 2048,
                "sources": [
                    {
                        "metric_kind": "peak_ram_bytes",
                        "source_kind": "os_process_rss"
                    },
                    {
                        "metric_kind": "peak_vram_bytes",
                        "source_kind": "pytorch_cuda"
                    }
                ]
            })
        );

        let decoded: InferenceExecutionResourceObservation =
            serde_json::from_value(encoded).expect("observation decodes");
        assert_eq!(decoded, observation);
    }

    #[test]
    fn resource_observation_rejects_zero_peak_values() {
        let error = InferenceExecutionResourceObservation::peak_ram(
            0,
            InferenceResourceObservationSourceKind::OsProcessRss,
        )
        .expect_err("zero peak ram should be rejected");

        assert_eq!(
            error,
            InferenceExecutionResourceObservationError::ZeroPeakValue {
                metric_kind: InferenceResourceObservationMetricKind::PeakRamBytes,
            }
        );
    }

    #[test]
    fn resource_observation_deduplicates_and_orders_bounded_facts() {
        let observation = InferenceExecutionResourceObservation::new(
            Some(4096),
            Some(2048),
            None,
            vec![
                source(
                    InferenceResourceObservationMetricKind::PeakVramBytes,
                    InferenceResourceObservationSourceKind::PytorchCuda,
                ),
                source(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                ),
                source(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                ),
            ],
            vec![
                unavailable(
                    InferenceResourceObservationMetricKind::PeakVramBytes,
                    InferenceResourceObservationUnavailableState::NotAvailable,
                    InferenceResourceObservationSourceKind::PytorchMps,
                ),
                unavailable(
                    InferenceResourceObservationMetricKind::PeakVramBytes,
                    InferenceResourceObservationUnavailableState::NotAvailable,
                    InferenceResourceObservationSourceKind::PytorchMps,
                ),
            ],
        )
        .expect("valid observation");

        assert_eq!(
            observation.sources(),
            &[
                source(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                ),
                source(
                    InferenceResourceObservationMetricKind::PeakVramBytes,
                    InferenceResourceObservationSourceKind::PytorchCuda,
                ),
            ]
        );
        assert_eq!(
            observation.availability(),
            &[unavailable(
                InferenceResourceObservationMetricKind::PeakVramBytes,
                InferenceResourceObservationUnavailableState::NotAvailable,
                InferenceResourceObservationSourceKind::PytorchMps,
            )]
        );
    }

    #[test]
    fn resource_observation_rejects_source_without_matching_fact() {
        let error = InferenceExecutionResourceObservation::new(
            Some(4096),
            None,
            None,
            vec![source(
                InferenceResourceObservationMetricKind::PeakVramBytes,
                InferenceResourceObservationSourceKind::PytorchCuda,
            )],
            Vec::new(),
        )
        .expect_err("source without matching metric should fail");

        assert_eq!(
            error,
            InferenceExecutionResourceObservationError::SourceWithoutMetricFact {
                metric_kind: InferenceResourceObservationMetricKind::PeakVramBytes,
            }
        );
    }

    #[test]
    fn resource_observation_rejects_unbounded_fact_collections() {
        let sources = (0..=INFERENCE_RESOURCE_OBSERVATION_SOURCE_LIMIT)
            .map(|_| {
                source(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    InferenceResourceObservationSourceKind::OsProcessRss,
                )
            })
            .collect();
        let error =
            InferenceExecutionResourceObservation::new(Some(4096), None, None, sources, Vec::new())
                .expect_err("source limit should be enforced before deduplication");

        assert_eq!(
            error,
            InferenceExecutionResourceObservationError::TooManySources {
                max_len: INFERENCE_RESOURCE_OBSERVATION_SOURCE_LIMIT,
                actual_len: INFERENCE_RESOURCE_OBSERVATION_SOURCE_LIMIT + 1,
            }
        );
    }

    #[test]
    fn resource_observation_deserialize_validates_contract() {
        let error =
            serde_json::from_value::<InferenceExecutionResourceObservation>(serde_json::json!({
                "peak_ram_bytes": 0
            }))
            .expect_err("wire decoding should validate observation invariants");

        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    fn resource_observation_merge_keeps_peak_max_and_failure_kind() {
        let peak_ram = InferenceExecutionResourceObservation::peak_ram(
            4096,
            InferenceResourceObservationSourceKind::OsProcessRss,
        )
        .expect("valid peak ram observation");
        let peak_vram = InferenceExecutionResourceObservation::new(
            Some(2048),
            Some(8192),
            Some(InferenceMemoryFailureKind::OutOfMemory),
            vec![
                source(
                    InferenceResourceObservationMetricKind::PeakRamBytes,
                    InferenceResourceObservationSourceKind::ManagedRuntimeTelemetry,
                ),
                source(
                    InferenceResourceObservationMetricKind::PeakVramBytes,
                    InferenceResourceObservationSourceKind::PytorchCuda,
                ),
            ],
            Vec::new(),
        )
        .expect("valid peak vram observation");

        let merged = peak_ram.merge(peak_vram).expect("observations merge");

        assert_eq!(merged.peak_ram_bytes(), Some(4096));
        assert_eq!(merged.peak_vram_bytes(), Some(8192));
        assert_eq!(
            merged.memory_failure_kind(),
            Some(InferenceMemoryFailureKind::OutOfMemory)
        );
        assert_eq!(merged.sources().len(), 3);
    }

    #[test]
    fn unavailable_resource_observation_round_trips_without_fake_values() {
        let observation = InferenceExecutionResourceObservation::unavailable(vec![unavailable(
            InferenceResourceObservationMetricKind::PeakVramBytes,
            InferenceResourceObservationUnavailableState::UnsupportedDevice,
            InferenceResourceObservationSourceKind::PytorchMps,
        )])
        .expect("availability-only observation is valid");

        let encoded = serde_json::to_value(&observation).expect("observation encodes");
        assert_eq!(encoded.get("peak_vram_bytes"), None);
        assert_eq!(
            encoded["availability"][0]["state"],
            serde_json::json!("unsupported_device")
        );

        let decoded: InferenceExecutionResourceObservation =
            serde_json::from_value(encoded).expect("observation decodes");
        assert_eq!(decoded, observation);
    }
}
