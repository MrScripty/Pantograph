//! Independent negative schema tests. These inert declarations are deliberately
//! not a reviewed provider and never confer positive provider/startup authority.
use super::{files_by_path, ComponentIdentity};
use pumas_app_manager::version_manager::TorchComponentFile;
use serde_json::{json, Value};

fn inert_identity() -> Value {
    let empty_sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    json!({
        "domain":"pumas.torch.component-assembly.v1", "policy_version":1,
        "base_tag":"inert-base", "base_manifest_sha256":empty_sha,
        "interpreter_depot_manifest_sha256":empty_sha,
        "qualification":"assembled_unqualified",
        "launch_restrictions":["no_runnable_interpreter_entry","no_initialized_import_provenance"],
        "component":{
            "distribution":"inert-fixture", "version":"0.0.0",
            "wheel_name":"inert-fixture.whl", "wheel_tag":"cp39-abi3-linux_x86_64",
            "python":"python3.12", "platform":"linux-x86_64",
            "archive_files":[{"path":"fixture.data","size":0,"sha256":empty_sha}],
            "replace_base_members":[],
            "final_dependency_files":[{"path":"fixture.data","size":0,"sha256":empty_sha}]
        },
        "input_manifest":{
            "schema_version":1,
            "source":{"provider":"fixture-local","source_id":"inert-input",
                "revision":{"authority":"fixture.sha256","value":empty_sha,"strength":"immutable"}},
            "files":[{"logical_path":"inert-fixture.whl","source_key":"inert-fixture.whl",
                "expected_size":0,"expected_sha256":{"authority":"fixture.sha256","value":empty_sha},
                "verification":"sha256"}]
        }
    })
}

fn rejects(value: Value) {
    assert!(serde_json::from_value::<ComponentIdentity>(value).is_err());
}

#[test]
fn provider_identity_unknown_fields_refuse_at_every_boundary() {
    let baseline = inert_identity();
    // Parsing an inert schema is not provider validation or a startup permit.
    assert!(serde_json::from_value::<ComponentIdentity>(baseline.clone()).is_ok());
    for pointer in [
        "",
        "/component",
        "/component/archive_files/0",
        "/component/final_dependency_files/0",
        "/input_manifest",
        "/input_manifest/source",
        "/input_manifest/source/revision",
        "/input_manifest/files/0",
        "/input_manifest/files/0/expected_sha256",
    ] {
        let mut altered = baseline.clone();
        altered
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("caller_ready_proof".into(), json!(true));
        rejects(altered);
    }
}

#[test]
fn provider_identity_missing_or_duplicate_fields_never_default() {
    let baseline = inert_identity();
    for field in [
        "domain",
        "policy_version",
        "base_tag",
        "base_manifest_sha256",
        "interpreter_depot_manifest_sha256",
        "qualification",
        "launch_restrictions",
        "component",
        "input_manifest",
    ] {
        let mut altered = baseline.clone();
        altered.as_object_mut().unwrap().remove(field);
        rejects(altered);
        let encoded = serde_json::to_string(&baseline).unwrap();
        // Construct actual duplicate wire keys, not a map that silently overwrites.
        let duplicate = format!("{{{field:?}:{},{}", baseline[field], &encoded[1..]);
        assert!(serde_json::from_str::<ComponentIdentity>(&duplicate).is_err());
    }
}

#[test]
fn provider_identity_acquisition_schema_and_digest_shape_are_not_bypassed() {
    let baseline = inert_identity();
    for (pointer, replacement) in [
        ("/input_manifest/schema_version", json!(2)),
        (
            "/input_manifest/files/0/logical_path",
            json!("../escape.whl"),
        ),
        (
            "/input_manifest/files/0/expected_sha256/value",
            json!("not-a-sha256"),
        ),
        ("/input_manifest/files/0/expected_sha256", Value::Null),
        (
            "/input_manifest/files/0/verification",
            json!("caller_ready"),
        ),
        (
            "/input_manifest/source/revision/strength",
            json!("caller_ready"),
        ),
        (
            "/input_manifest/source/provider",
            json!("https://example.invalid"),
        ),
    ] {
        let mut altered = baseline.clone();
        *altered.pointer_mut(pointer).unwrap() = replacement;
        rejects(altered);
    }
    let mut duplicate_file = baseline;
    let files = duplicate_file["input_manifest"]["files"]
        .as_array_mut()
        .unwrap();
    files.push(files[0].clone());
    rejects(duplicate_file);
}

fn member(path: &str, size: u64) -> TorchComponentFile {
    TorchComponentFile {
        path: path.into(),
        size,
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
    }
}

#[test]
fn provider_files_preserve_exact_case_and_refuse_casefold_duplicates() {
    let files = [member("Package/Member.DAT", 0), member("other.data", 1)];
    let map = files_by_path(&files).unwrap();
    assert!(map.contains_key("Package/Member.DAT"));
    assert!(!map.contains_key("package/member.dat"));
    for duplicate in [
        "Package/Member.DAT",
        "package/member.dat",
        "PACKAGE/MEMBER.DAT",
    ] {
        assert!(files_by_path(&[files[0].clone(), member(duplicate, 0)]).is_err());
    }
}

#[test]
fn provider_files_refuse_path_and_digest_ambiguity_before_association() {
    for path in [
        "",
        "/absolute",
        "../escape",
        "a/../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
        "nonascii-é",
    ] {
        assert!(files_by_path(&[member(path, 0)]).is_err(), "path {path:?}");
    }
    for hash in [
        "",
        "a",
        "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ] {
        let mut file = member("file.data", 0);
        file.sha256 = hash.into();
        assert!(files_by_path(&[file]).is_err());
    }
}

#[test]
fn provider_files_exact_limits_zero_and_extreme_sizes_are_distinct() {
    assert!(files_by_path(&[]).is_err());
    assert!(files_by_path(&[member("zero.data", 0)]).is_ok());
    assert!(files_by_path(&[member(&"a".repeat(1024), 0)]).is_ok());
    assert!(files_by_path(&[member(&"a".repeat(1025), 0)]).is_err());
    const PER_FILE: u64 = 2 * 1024 * 1024 * 1024;
    assert!(files_by_path(&[member("one.data", PER_FILE)]).is_ok());
    assert!(files_by_path(&[member("one.data", PER_FILE + 1)]).is_err());
    assert!(files_by_path(&[member("one.data", u64::MAX)]).is_err());
    let exact = [member("one.data", PER_FILE), member("two.data", PER_FILE)];
    assert!(files_by_path(&exact).is_ok());
    assert!(files_by_path(&[exact[0].clone(), exact[1].clone(), member("extra.data", 1)]).is_err());
    // No files or native bytes allocated: these are declaration thresholds only.
}
