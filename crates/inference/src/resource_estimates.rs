use serde::{Deserialize, Serialize};

/// Resource estimate kinds produced by inference/backend planning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceResourceEstimateKind {
    OutputRgbaBytes,
    VaeWorkingMemoryBytes,
    ModelResidencyBytes,
    RuntimeOverheadBytes,
    PeakVramBytes,
    PeakRamBytes,
}

/// Explicit state for an estimate; missing facts are not encoded as sentinel values.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceResourceEstimateState {
    Available,
    NotAvailable,
    NotImplemented,
    InsufficientFacts,
    Overflow,
    UnsupportedFamily,
    UnsupportedRuntime,
}

impl InferenceResourceEstimateState {
    #[must_use]
    pub fn is_available(self) -> bool {
        self == Self::Available
    }
}

/// Stable diagnostic code for resource-estimate production.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceResourceEstimateDiagnosticCode {
    ArithmeticOverflow,
    InvalidInput,
    InsufficientFacts,
    NotAvailable,
    NotImplemented,
    UnsupportedFamily,
    UnsupportedRuntime,
}

/// Diagnostic severity for resource-estimate production.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceResourceEstimateDiagnosticSeverity {
    Error,
}

/// Bounded diagnostic attached to a non-available resource estimate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceResourceEstimateDiagnostic {
    pub code: InferenceResourceEstimateDiagnosticCode,
    pub severity: InferenceResourceEstimateDiagnosticSeverity,
    pub field_path: String,
    pub message: String,
}

impl InferenceResourceEstimateDiagnostic {
    #[must_use]
    pub fn error(
        code: InferenceResourceEstimateDiagnosticCode,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: InferenceResourceEstimateDiagnosticSeverity::Error,
            field_path: field_path.into(),
            message: message.into(),
        }
    }
}

/// Error returned when constructing an invalid estimate contract value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceResourceEstimateError {
    #[error("available resource estimates require a value")]
    AvailableEstimateMissingValue,
    #[error("available resource estimates must not carry diagnostics")]
    AvailableEstimateHasDiagnostics,
    #[error("non-available resource estimates must not carry a value")]
    NonAvailableEstimateHasValue,
    #[error("non-available resource estimates must not use the available state")]
    NonAvailableEstimateUsesAvailableState,
}

/// Resource estimate with an explicit state and optional byte value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceResourceEstimate {
    kind: InferenceResourceEstimateKind,
    state: InferenceResourceEstimateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<InferenceResourceEstimateDiagnostic>,
}

impl InferenceResourceEstimate {
    pub fn available(kind: InferenceResourceEstimateKind, value_bytes: u64) -> Self {
        Self {
            kind,
            state: InferenceResourceEstimateState::Available,
            value_bytes: Some(value_bytes),
            diagnostics: Vec::new(),
        }
    }

    pub fn unavailable(
        kind: InferenceResourceEstimateKind,
        state: InferenceResourceEstimateState,
        diagnostics: Vec<InferenceResourceEstimateDiagnostic>,
    ) -> Result<Self, InferenceResourceEstimateError> {
        if state.is_available() {
            return Err(InferenceResourceEstimateError::NonAvailableEstimateUsesAvailableState);
        }
        Ok(Self {
            kind,
            state,
            value_bytes: None,
            diagnostics,
        })
    }

    #[must_use]
    pub fn kind(&self) -> InferenceResourceEstimateKind {
        self.kind
    }

    #[must_use]
    pub fn state(&self) -> InferenceResourceEstimateState {
        self.state
    }

    #[must_use]
    pub fn value_bytes(&self) -> Option<u64> {
        self.value_bytes
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[InferenceResourceEstimateDiagnostic] {
        &self.diagnostics
    }

    pub fn validate(&self) -> Result<(), InferenceResourceEstimateError> {
        if self.state.is_available() {
            if self.value_bytes.is_none() {
                return Err(InferenceResourceEstimateError::AvailableEstimateMissingValue);
            }
            if !self.diagnostics.is_empty() {
                return Err(InferenceResourceEstimateError::AvailableEstimateHasDiagnostics);
            }
            return Ok(());
        }

        if self.value_bytes.is_some() {
            return Err(InferenceResourceEstimateError::NonAvailableEstimateHasValue);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_resource_estimate_round_trips_with_value() {
        let estimate =
            InferenceResourceEstimate::available(InferenceResourceEstimateKind::OutputRgbaBytes, 4);

        let encoded = serde_json::to_value(&estimate).expect("estimate should encode");
        assert_eq!(
            encoded,
            serde_json::json!({
                "kind": "output_rgba_bytes",
                "state": "available",
                "value_bytes": 4
            })
        );

        let decoded: InferenceResourceEstimate =
            serde_json::from_value(encoded).expect("estimate should decode");
        assert_eq!(decoded, estimate);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn unavailable_resource_estimate_round_trips_without_sentinel_value() {
        let diagnostic = InferenceResourceEstimateDiagnostic::error(
            InferenceResourceEstimateDiagnosticCode::ArithmeticOverflow,
            "request.width/request.height",
            "output byte estimate overflowed",
        );
        let estimate = InferenceResourceEstimate::unavailable(
            InferenceResourceEstimateKind::OutputRgbaBytes,
            InferenceResourceEstimateState::Overflow,
            vec![diagnostic],
        )
        .expect("overflow estimate should build");

        let encoded = serde_json::to_value(&estimate).expect("estimate should encode");
        assert_eq!(encoded.get("value_bytes"), None);
        assert_eq!(encoded.get("state"), Some(&serde_json::json!("overflow")));

        let decoded: InferenceResourceEstimate =
            serde_json::from_value(encoded).expect("estimate should decode");
        assert_eq!(decoded, estimate);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn non_available_resource_estimate_rejects_available_state() {
        let error = InferenceResourceEstimate::unavailable(
            InferenceResourceEstimateKind::PeakVramBytes,
            InferenceResourceEstimateState::Available,
            Vec::new(),
        )
        .expect_err("available state is not a non-available estimate");

        assert_eq!(
            error,
            InferenceResourceEstimateError::NonAvailableEstimateUsesAvailableState
        );
    }
}
