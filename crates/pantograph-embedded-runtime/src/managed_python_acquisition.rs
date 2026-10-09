//! Pure reviewed wheel selection for the existing Pumas acquisition boundary.
//!
//! A manifest supplies selection metadata, never actual input or runtime custody.
//! Preparing it opens no file, submits no acquisition and permits no native import.
use inference::managed_python_binding::{
    TOKENIZER_PROVIDER_BUILD_BINDING_SHA256, TOKENIZER_PROVIDER_WHEEL_FILENAME,
    TOKENIZER_PROVIDER_WHEEL_SHA256, TOKENIZER_PROVIDER_WHEEL_SIZE_BYTES,
};
use pumas_library::acquisition::{
    ArtifactFile, ArtifactManifest, ArtifactRevisionEvidence, ArtifactSourceIdentity,
    FileVerificationRequirement, ManifestValidationError, RevisionStrength, Sha256Evidence,
};

/// Prepare the sole reviewed wheel member with mandatory exact size and SHA256.
///
/// The revision identifies the frozen prepared-source/build association, including
/// all nine overlay hashes; an upstream tag alone would omit those modifications.
/// Immutable describes that reviewed selection, not a caller's file or runtime.
///
/// The manifest API already exists in the pinned Pumas26a and is unchanged in
/// published c9. A later approved adapter can supply this selection to
/// `AcquisitionLocalRequest`, alongside actual held inputs and existing owner
/// workspace/demand/retry custody. This function constructs none of those grants.
pub fn reviewed_tokenizer_acquisition_manifest() -> Result<ArtifactManifest, ManifestValidationError>
{
    let source = ArtifactSourceIdentity::new(
        "pantograph-local-component",
        "tokenizers-0.21.4+pantograph.snapshot1",
        ArtifactRevisionEvidence::new(
            "pantograph.build-source-binding.sha256",
            TOKENIZER_PROVIDER_BUILD_BINDING_SHA256,
            RevisionStrength::Immutable,
        )?,
    )?;
    let wheel = ArtifactFile::new(
        TOKENIZER_PROVIDER_WHEEL_FILENAME,
        TOKENIZER_PROVIDER_WHEEL_FILENAME,
        Some(TOKENIZER_PROVIDER_WHEEL_SIZE_BYTES),
        Some(Sha256Evidence::new(
            "pantograph.reviewed-local-wheel.sha256",
            TOKENIZER_PROVIDER_WHEEL_SHA256,
        )?),
        FileVerificationRequirement::Sha256,
    )?;
    ArtifactManifest::new(source, vec![wheel])
}

#[cfg(test)]
#[path = "managed_python_acquisition_tests.rs"]
mod tests;
