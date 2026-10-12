//! Observes the published owner contract without a Pantograph adapter or loader.
use super::{copy_fixture, json, save, seed_fixture, Path, PathBuf, ProbeResult, Value};
use pumas_library::models::{
    PackageArtifactKind, PumasArtifactConsumer, PumasArtifactLoadTargetResolutionMode,
    PumasModelRef, ResolveModelArtifactLoadTargetRequest,
};

const COMPLETE_FILES: &[&str] = &[
    "model.safetensors",
    "config.json",
    "tokenizer.json",
    "modules.json",
    "sentence_bert_config.json",
    "1_Pooling/config.json",
];

fn redact(value: &mut Value, root: &Path) {
    match value {
        Value::String(text) => *text = text.replace(&*root.to_string_lossy(), "<probe_root>"),
        Value::Array(items) => items.iter_mut().for_each(|item| redact(item, root)),
        Value::Object(fields) => fields.values_mut().for_each(|item| redact(item, root)),
        _ => {}
    }
}

fn path_facts(path: &Path) -> Value {
    match std::fs::metadata(path) {
        Ok(metadata) => json!({"path":path, "absolute":path.is_absolute(), "exists":true,
            "is_directory":metadata.is_dir(), "is_file":metadata.is_file(), "bytes":metadata.len()}),
        Err(error) => json!({"path":path, "absolute":path.is_absolute(), "exists":false,
            "error_kind":format!("{:?}", error.kind())}),
    }
}

async fn observe(api: &pumas_library::PumasApi, root: &Path, model_id: &str) -> ProbeResult<Value> {
    let descriptor = api.resolve_model_execution_descriptor(model_id).await?;
    let facts = api.resolve_model_package_facts(model_id).await?;
    let request = ResolveModelArtifactLoadTargetRequest {
        model_ref: PumasModelRef {
            model_id: model_id.into(),
            selected_artifact_id: facts.model_ref.selected_artifact_id.clone(),
            ..Default::default()
        },
        expected_artifact_kind: None,
        caller_observed_entry_path: None,
        caller_observed_package_facts_contract_version: None,
        resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
        consumer: PumasArtifactConsumer {
            consumer_name: "pantograph-embedded-runtime".into(),
            task_kind: Some("embedding".into()),
            runtime_family: Some("candle.cpu".into()),
        },
    };
    let owner = api
        .resolve_model_artifact_load_target(request.clone())
        .await?;
    let mut indexed_request = request.clone();
    indexed_request.resolution_mode = PumasArtifactLoadTargetResolutionMode::ReadOnlyIndexed;
    let indexed = api
        .resolve_model_artifact_load_target(indexed_request)
        .await?;
    let mut strict_request = request.clone();
    strict_request.expected_artifact_kind = Some(PackageArtifactKind::HfCompatibleDirectory);
    let strict = api
        .resolve_model_artifact_load_target(strict_request)
        .await?;
    let model_dir = root.join("shared-resources/models").join(model_id);
    let target_path = owner
        .target
        .as_ref()
        .map(|target| path_facts(Path::new(&target.local_load_path)));
    let components: Vec<_> = COMPLETE_FILES
        .iter()
        .map(|name| path_facts(&model_dir.join(name)))
        .collect();
    Ok(json!({
        "model_id":model_id, "indexed_record":api.get_model(model_id).await?,
        "execution_descriptor_response":descriptor, "package_facts_response":facts,
        "load_target_request":request, "owner_fresh_response":owner,
        "read_only_indexed_response":indexed, "expected_kind_response":strict,
        "filesystem":{"model_root":path_facts(&model_dir), "returned_target":target_path, "components":components},
    }))
}

pub(super) async fn inspect(root: &Path) -> ProbeResult<()> {
    for directory in [
        "launcher-data/metadata",
        "launcher-data/cache",
        "launcher-data/logs",
    ] {
        std::fs::create_dir_all(root.join(directory))?;
    }
    let selections = [
        ("weights-only", Some(&["model.safetensors"][..])),
        ("complete-files", Some(COMPLETE_FILES)),
        ("owner-inferred", None),
    ];
    let mut models = Vec::new();
    for (name, selection) in selections {
        let id = format!("embedding/qualification/{name}");
        seed_fixture(root, &id, selection)?;
        models.push((name, id));
    }
    let api = pumas_library::PumasApi::builder(root)
        .with_hf_client(false)
        .with_process_manager(false)
        .build()
        .await?;
    api.rebuild_model_index().await?;
    let mut cases = Vec::new();
    for (name, id) in models {
        cases.push(
            json!({"setup":"seeded library-owned metadata + rebuild_model_index",
            "case":name, "observation":observe(&api, root, &id).await?}),
        );
    }

    // A fourth case uses Pumas's own in-place importer with no seeded metadata.
    let imported_dir = root.join("shared-resources/models/embedding/qualification/imported-bert-8");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../inference/tests/fixtures/candle_bert/bert-8");
    copy_fixture(&source, &imported_dir)?;
    let import_request = pumas_library::model_library::InPlaceImportSpec {
        model_dir: imported_dir,
        official_name: "Imported BERT 8".into(),
        family: "qualification".into(),
        model_type: Some("embedding".into()),
        repo_id: None,
        download_request: None,
        known_sha256: None,
        compute_hashes: true,
        expected_files: Some(
            COMPLETE_FILES
                .iter()
                .map(|file| (*file).to_owned())
                .collect(),
        ),
        pipeline_tag: Some("feature-extraction".into()),
        huggingface_evidence: None,
        release_date: None,
        download_url: None,
        model_card_json: None,
        license_status: None,
    };
    let imported = api.import_model_in_place(&import_request).await;
    match imported {
        Ok(response) => {
            let observation = if response.success {
                Some(
                    observe(
                        &api,
                        root,
                        response
                            .model_id
                            .as_deref()
                            .ok_or("import missing model id")?,
                    )
                    .await?,
                )
            } else {
                None
            };
            cases.push(json!({"case":"owner-imported", "setup":"public import_model_in_place; no seeded metadata",
                "import_request":import_request, "import_response":response, "observation":observation}));
        }
        Err(error) => cases.push(
            json!({"case":"owner-imported", "import_request":import_request,
            "import_error":error.to_string()}),
        ),
    }
    let pumas_lock_source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"))
        .split("[[package]]")
        .find(|package| {
            package
                .lines()
                .any(|line| line == "name = \"pumas-library\"")
        })
        .and_then(|package| {
            package
                .lines()
                .find_map(|line| line.strip_prefix("source = \""))
        })
        .and_then(|source| source.strip_suffix('"'))
        .ok_or("Pumas git source missing from compile-time Cargo.lock")?;
    let mut report = json!({"scope":"provider_contract_observation_only", "editor_qualified":false,
        "pumas_lock_source":pumas_lock_source, "cases":cases});
    redact(&mut report, root);
    save(root, "pumas-target-observations.json", &report)?;
    println!(
        "Saved redacted Pumas target observations. This does not qualify inference or the editor."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_paths_are_redacted_without_dropping_response_fields() {
        let mut value = json!({"target":{"local_load_path":"/tmp/probe/model.safetensors",
            "load_path_kind":"directory"}, "paths":["/tmp/probe"], "ready":true});
        redact(&mut value, Path::new("/tmp/probe"));
        assert_eq!(
            value["target"]["local_load_path"],
            "<probe_root>/model.safetensors"
        );
        assert_eq!(value["target"]["load_path_kind"], "directory");
        assert_eq!(value["paths"][0], "<probe_root>");
        assert_eq!(value["ready"], true);
    }

    #[test]
    fn filesystem_observation_distinguishes_directory_file_and_absence() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("model.safetensors");
        std::fs::write(&file, b"observation only").unwrap();
        assert_eq!(path_facts(temp.path())["is_directory"], true);
        assert_eq!(path_facts(&file)["is_directory"], false);
        assert_eq!(path_facts(&file)["is_file"], true);
        assert_eq!(path_facts(&temp.path().join("absent"))["exists"], false);
    }
}
