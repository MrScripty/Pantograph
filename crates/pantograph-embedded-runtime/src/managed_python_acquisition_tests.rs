//! Independent pure selection-metadata tests; no acquisition or file access.
use super::reviewed_tokenizer_acquisition_manifest;
use pumas_library::acquisition::{
    ArtifactManifest, FileVerificationRequirement, RevisionStrength, CURRENT_MANIFEST_VERSION,
};
use serde_json::json;

#[test]
fn reviewed_local_manifest_pins_exact_sole_wheel_and_prepared_build_revision() {
    let one = reviewed_tokenizer_acquisition_manifest().unwrap();
    let two = reviewed_tokenizer_acquisition_manifest().unwrap();
    assert_eq!(one, two);
    assert_eq!(one.schema_version(), CURRENT_MANIFEST_VERSION);
    assert_eq!(one.source().provider(), "pantograph-local-component");
    assert_eq!(
        one.source().source_id(),
        "tokenizers-0.21.4+pantograph.snapshot1"
    );
    assert_eq!(
        one.source().revision().strength(),
        RevisionStrength::Immutable
    );
    assert_eq!(
        one.source().revision().authority(),
        "pantograph.build-source-binding.sha256"
    );
    assert_eq!(
        one.source().revision().value(),
        "b2a4fc2a43b6ae5631b2f5ec45deb5394cd2059e456f97213a87d1bc5d1f703a"
    );
    assert_eq!(one.files().len(), 1);
    assert_eq!(one.total_bytes(), Some(24_366_281));
    let file = &one.files()[0];
    let filename = "tokenizers-0.21.4+pantograph.snapshot1-cp39-abi3-linux_x86_64.whl";
    assert_eq!(file.logical_path(), filename);
    assert_eq!(file.source_key(), filename);
    assert_eq!(file.expected_size(), Some(24_366_281));
    assert_eq!(file.verification(), FileVerificationRequirement::Sha256);
    assert_eq!(
        file.expected_sha256().unwrap().authority(),
        "pantograph.reviewed-local-wheel.sha256"
    );
    assert_eq!(
        file.expected_sha256().unwrap().value(),
        "678b155145bb06c271ad6d8eb2df95a8a8173155323fafd4b102e50349940b95"
    );
    assert!(one.requires_file_verification(0));
    assert!(one.permits_resume(0));
    assert!(!one.requires_file_verification(1));
    assert!(!one.permits_resume(1));
    let wire = serde_json::to_vec(&one).unwrap();
    assert_eq!(wire, serde_json::to_vec(&two).unwrap());
    assert_eq!(
        serde_json::from_slice::<ArtifactManifest>(&wire).unwrap(),
        one
    );
}

#[test]
fn reviewed_local_manifest_schema_and_path_validation_cannot_serialize_custody() {
    let value = reviewed_tokenizer_acquisition_manifest().unwrap();
    let baseline = serde_json::to_value(value).unwrap();
    for (field, fake) in [
        ("owner_lease", json!({"held": true})),
        ("verified", json!(true)),
        ("runtime_registered", json!(true)),
        ("input_source", json!("/tmp/pretend-wheel")),
    ] {
        let mut altered = baseline.clone();
        altered[field] = fake;
        assert!(
            serde_json::from_value::<ArtifactManifest>(altered).is_err(),
            "{field}"
        );
    }
    for path in ["../escape.whl", "/absolute.whl", "a/../escape.whl"] {
        let mut altered = baseline.clone();
        altered["files"][0]["logical_path"] = json!(path);
        assert!(
            serde_json::from_value::<ArtifactManifest>(altered).is_err(),
            "{path}"
        );
    }
    let mut missing_digest = baseline.clone();
    missing_digest["files"][0]["expected_sha256"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ArtifactManifest>(missing_digest).is_err());
    let mut duplicate = baseline.clone();
    let repeated = duplicate["files"][0].clone();
    duplicate["files"].as_array_mut().unwrap().push(repeated);
    assert!(serde_json::from_value::<ArtifactManifest>(duplicate).is_err());
    let mut empty = baseline.clone();
    empty["files"] = json!([]);
    assert!(serde_json::from_value::<ArtifactManifest>(empty).is_err());
    let mut unknown_version = baseline;
    unknown_version["schema_version"] = json!(u16::MAX);
    assert!(serde_json::from_value::<ArtifactManifest>(unknown_version).is_err());
}

#[test]
fn reviewed_local_manifest_requirements_are_declarations_not_verified_bytes_or_permission() {
    let reviewed = reviewed_tokenizer_acquisition_manifest().unwrap();
    let mut different_declaration = serde_json::to_value(&reviewed).unwrap();
    different_declaration["files"][0]["expected_size"] = json!(1);
    different_declaration["files"][0]["expected_sha256"]["value"] = json!("0".repeat(64));
    // The existing owner type validates shape, deliberately not truth or authority.
    // A syntactically valid alternative must never be confused with this helper's
    // immutable reviewed selection or treated as an acquired/registered input.
    let declaration: ArtifactManifest = serde_json::from_value(different_declaration).unwrap();
    assert_ne!(declaration, reviewed);
    assert_eq!(declaration.total_bytes(), Some(1));
    assert_eq!(
        declaration.files()[0].expected_sha256().unwrap().value(),
        "0".repeat(64)
    );
    assert!(declaration.requires_file_verification(0));
    assert_eq!(reviewed_tokenizer_acquisition_manifest().unwrap(), reviewed);
    let serialized = serde_json::to_value(&reviewed).unwrap();
    assert_eq!(serialized.as_object().unwrap().len(), 3);
    assert!(serialized.get("verified").is_none());
    assert!(serialized.get("owner_lease").is_none());
    assert!(serialized.get("runtime_registered").is_none());
}
