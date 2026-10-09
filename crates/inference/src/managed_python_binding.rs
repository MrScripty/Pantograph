//! Managed Python startup request and the current, deliberately closed boundary.
//!
//! This request identifies reviewed inputs; it supplies no owner lease, import
//! provenance or timing authority. Pumas26a has no local-component/interpreter
//! lease API. Every managed start therefore refuses before backend effects.
use crate::{CapabilityAvailabilityId, RuntimeVariantId};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MANAGED_PYTHON_START_CONTRACT_VERSION: u32 = 1;
pub const TOKENIZER_PROVIDER_WHEEL_FILENAME: &str =
    "tokenizers-0.21.4+pantograph.snapshot1-cp39-abi3-linux_x86_64.whl";
pub const TOKENIZER_PROVIDER_WHEEL_SIZE_BYTES: u64 = 24_366_281;
pub const TOKENIZER_PROVIDER_WHEEL_SHA256: &str =
    "678b155145bb06c271ad6d8eb2df95a8a8173155323fafd4b102e50349940b95";
pub const TOKENIZER_PROVIDER_EXTENSION_SHA256: &str =
    "5a7eacfad202bcacf122716f5a45e9796d098c8e65f86a56536efddceb573423";
pub const TOKENIZER_PROVIDER_BUILD_BINDING_SHA256: &str =
    "b2a4fc2a43b6ae5631b2f5ec45deb5394cd2059e456f97213a87d1bc5d1f703a";

/// Metadata only. Serialized declarations cannot provide execution custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedPythonRuntimeStartRequest {
    pub contract_version: u32,
    pub environment_id: CapabilityAvailabilityId,
    /// Opaque owner revision; never a path, URL or active-version alias.
    pub runtime_revision: String,
    pub runtime_manifest_sha256: String,
    pub runtime_variant_id: RuntimeVariantId,
    pub provider_wheel_sha256: String,
    pub provider_extension_sha256: String,
    pub provider_build_binding_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedPythonBindingRefusal {
    InvalidStartRequest,
    LegacyWorkerAlreadyInitialized,
    RegisteredOwnerCustodyUnavailable,
}

impl fmt::Display for ManagedPythonBindingRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidStartRequest => "managed Python start request is invalid or unqualified",
            Self::LegacyWorkerAlreadyInitialized => {
                "managed Python cannot adopt the initialized legacy shared worker"
            }
            Self::RegisteredOwnerCustodyUnavailable => {
                "managed Python requires a Pumas immutable bundle lease and prior interpreter/import registration"
            }
        })
    }
}
impl std::error::Error for ManagedPythonBindingRefusal {}

/// Negative observations only; these flags cannot authorize a positive binding.
#[derive(Debug, Clone, Copy)]
#[cfg(any(test, feature = "backend-pytorch"))]
pub(crate) struct ManagedPythonStartObservation {
    pub(crate) legacy_worker_initialized: bool,
}

impl ManagedPythonRuntimeStartRequest {
    pub fn validate(&self) -> Result<(), ManagedPythonBindingRefusal> {
        let revision = self.runtime_revision.as_bytes();
        let manifest = self.runtime_manifest_sha256.as_bytes();
        if self.contract_version != MANAGED_PYTHON_START_CONTRACT_VERSION
            || revision.is_empty()
            || revision.len() > 256
            || !revision.iter().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'-' | b'_' | b'+')
            })
            || manifest.len() != 64
            || !manifest
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
            || self.runtime_variant_id.as_str() != "pytorch.cpu"
            || self.provider_wheel_sha256 != TOKENIZER_PROVIDER_WHEEL_SHA256
            || self.provider_extension_sha256 != TOKENIZER_PROVIDER_EXTENSION_SHA256
            || self.provider_build_binding_sha256 != TOKENIZER_PROVIDER_BUILD_BINDING_SHA256
        {
            return Err(ManagedPythonBindingRefusal::InvalidStartRequest);
        }
        Ok(())
    }

    /// Entry-before-effects preflight. Validate metadata before even querying
    /// existing interpreter/worker flags. No positive variant is fabricated:
    /// mutable sys/module labels or a caller-declared lease cannot open this gate.
    #[cfg(any(test, feature = "backend-pytorch"))]
    pub(crate) fn start_refusal(
        &self,
        observe: impl FnOnce() -> ManagedPythonStartObservation,
    ) -> ManagedPythonBindingRefusal {
        if self.validate().is_err() {
            return ManagedPythonBindingRefusal::InvalidStartRequest;
        }
        let observed = observe();
        if observed.legacy_worker_initialized {
            return ManagedPythonBindingRefusal::LegacyWorkerAlreadyInitialized;
        }
        ManagedPythonBindingRefusal::RegisteredOwnerCustodyUnavailable
    }
}

#[cfg(test)]
#[path = "managed_python_binding_tests.rs"]
mod tests;
