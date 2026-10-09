//! Static association of reviewed provider digests with genuine retained bytes.
//! This checks the published assembly format; no wheel/interpreter is executed,
//! no build history is authenticated, and no startup permission is returned.
use crate::managed_python_binding::*;
use pumas_app_manager::version_manager::{
    TorchComponentFile, TorchComponentManifest, TorchComponentSelection,
};
use pumas_byte_custody::{
    acquisition::{ArtifactManifest, FileVerificationRequirement, RevisionStrength},
    runtime_read_source::RuntimeReadRole,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FILES: usize = 200_000;
const EXTENSION: &str = "tokenizers/tokenizers.abi3.so";
const DIST_INFO: &str = "tokenizers-0.21.4+pantograph.snapshot1.dist-info";
const BINDING: &str =
    "tokenizers-0.21.4+pantograph.snapshot1.dist-info/pantograph-build-source-binding.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentIdentity {
    domain: String,
    policy_version: u32,
    base_tag: String,
    base_manifest_sha256: String,
    interpreter_depot_manifest_sha256: String,
    qualification: String,
    launch_restrictions: Vec<String>,
    component: TorchComponentManifest,
    input_manifest: ArtifactManifest,
}

pub(crate) fn validate(selection: &TorchComponentSelection) -> Result<(), ()> {
    // This is deliberately blocking and keeps the selection alive on the stack.
    // Owner validation checks the whole final closure; clone_member rechecks the
    // concrete descriptor association. Neither call holds the broker mutex.
    selection.validate().map_err(|_| ())?;
    let source = selection.retained_source();
    let mut file = source
        .clone_member(RuntimeReadRole::Sidecar, "component-manifest.json")
        .map_err(|_| ())?;
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES
        || format!("{:x}", Sha256::digest(&bytes)) != selection.manifest_sha256()
    {
        return Err(());
    }
    let identity: ComponentIdentity = serde_json::from_slice(&bytes).map_err(|_| ())?;
    validate_identity(&identity, selection)?;
    // Reopen required native/binding members through custody, rather than any
    // ambient path. The owner's clone_member streams size/SHA verification.
    source
        .clone_member(RuntimeReadRole::Dependencies, EXTENSION)
        .map_err(|_| ())?;
    source
        .clone_member(RuntimeReadRole::Dependencies, BINDING)
        .map_err(|_| ())?;
    Ok(())
}

fn canonical_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn files_by_path(files: &[TorchComponentFile]) -> Result<BTreeMap<String, (u64, &str)>, ()> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err(());
    }
    let mut map = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut total = 0_u64;
    for file in files {
        if file.path.is_empty()
            || file.path.len() > 1024
            || !file.path.split('/').all(|part| {
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
            })
            || !canonical_digest(&file.sha256)
            || file.size > 2 * 1024 * 1024 * 1024
            || !names.insert(file.path.to_ascii_lowercase())
        {
            return Err(());
        }
        map.insert(file.path.clone(), (file.size, file.sha256.as_str()));
        total = total.checked_add(file.size).ok_or(())?;
        if total > 4 * 1024 * 1024 * 1024 {
            return Err(());
        }
    }
    Ok(map)
}

fn validate_identity(
    identity: &ComponentIdentity,
    selection: &TorchComponentSelection,
) -> Result<(), ()> {
    let component = &identity.component;
    let input = &identity.input_manifest;
    let source = input.source();
    let revision = source.revision();
    if identity.domain != "pumas.torch.component-assembly.v1"
        || identity.policy_version != 1
        || identity.base_tag.is_empty()
        || identity.base_tag.len() > 256
        || !canonical_digest(&identity.base_manifest_sha256)
        || identity.interpreter_depot_manifest_sha256
            != selection.interpreter_depot_manifest_sha256()
        || identity.qualification != "assembled_unqualified"
        || identity.launch_restrictions
            != [
                "no_runnable_interpreter_entry",
                "no_initialized_import_provenance",
            ]
        || component.distribution != "tokenizers"
        || component.version != "0.21.4+pantograph.snapshot1"
        || component.wheel_name != TOKENIZER_PROVIDER_WHEEL_FILENAME
        || component.wheel_tag != "cp39-abi3-linux_x86_64"
        || component.python != "python3.12"
        || component.platform != "linux-x86_64"
        || source.provider() != "pantograph-local-component"
        || source.source_id() != "tokenizers-0.21.4+pantograph.snapshot1"
        || revision.authority() != "pantograph.build-source-binding.sha256"
        || revision.value() != TOKENIZER_PROVIDER_BUILD_BINDING_SHA256
        || revision.strength() != RevisionStrength::Immutable
        || input.files().len() != 1
    {
        return Err(());
    }
    let wheel = &input.files()[0];
    if wheel.logical_path() != TOKENIZER_PROVIDER_WHEEL_FILENAME
        || wheel.source_key() != TOKENIZER_PROVIDER_WHEEL_FILENAME
        || wheel.expected_size() != Some(TOKENIZER_PROVIDER_WHEEL_SIZE_BYTES)
        || wheel.verification() != FileVerificationRequirement::Sha256
        || !wheel.expected_sha256().is_some_and(|sha| {
            sha.authority() == "pantograph.reviewed-local-wheel.sha256"
                && sha.value() == TOKENIZER_PROVIDER_WHEEL_SHA256
        })
    {
        return Err(());
    }
    let archive = files_by_path(&component.archive_files)?;
    let final_files = files_by_path(&component.final_dependency_files)?;
    // Exact final closure belongs to Pumas; base dependencies may legitimately
    // remain outside the archive. Every archive member must survive unchanged.
    if archive
        .iter()
        .any(|(path, facts)| final_files.get(path) != Some(facts))
    {
        return Err(());
    }
    let actual: BTreeMap<_, _> = selection
        .retained_source()
        .manifest()
        .filter(|(role, _)| *role == RuntimeReadRole::Dependencies)
        .map(|(_, file)| (file.path().to_owned(), (file.size(), file.sha256())))
        .collect();
    if actual != final_files {
        return Err(());
    }
    if archive.get(EXTENSION)
        != Some(&(
            TOKENIZER_PROVIDER_EXTENSION_SIZE_BYTES,
            TOKENIZER_PROVIDER_EXTENSION_SHA256,
        ))
        || !archive.get(BINDING).is_some_and(|(size, sha)| {
            *size > 0
                && *size <= MAX_MANIFEST_BYTES
                && *sha == TOKENIZER_PROVIDER_BUILD_BINDING_SHA256
        })
        || ["METADATA", "WHEEL", "RECORD"]
            .iter()
            .any(|name| !archive.contains_key(&format!("{DIST_INFO}/{name}")))
        || final_files.keys().any(|path| {
            path.to_ascii_lowercase()
                .split('/')
                .next()
                .is_some_and(|root| {
                    root.starts_with("tokenizers-")
                        && root.ends_with(".dist-info")
                        && root != DIST_INFO
                })
        })
        || final_files.keys().any(|path| {
            let lower = path.to_ascii_lowercase();
            lower.starts_with("tokenizers/") && lower.ends_with(".so") && path != EXTENSION
        })
    {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
#[path = "pumas_provider_association_tests.rs"]
mod tests;
