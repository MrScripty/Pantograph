use super::{
    DownloadResponseMode, ManagedBinaryCapability, ManagedBinaryId, ManagedBinaryInstallState,
    ManagedRuntimeJobState, ManagedRuntimeJobStatus, ManagedRuntimeReadinessState,
    binary_capability, cancel_binary_download, definition, download_response_mode,
    ensure_runtime_state_entry, existing_download_artifact, finish_requested_cancellation,
    finish_requested_pause, pause_binary_download, persist_install_success, persist_remove_success,
    readiness_state_for_capability, resolve_runtime_install_dir,
    runtime_install_dir_for_projection, select_managed_runtime_version,
    set_default_managed_runtime_version, snapshot_from_capability,
};
use crate::managed_runtime::{
    ManagedRuntimeHistoryEventKind, ManagedRuntimePersistedJobArtifact,
    ManagedRuntimePersistedVersion, load_managed_runtime_state, save_managed_runtime_state,
};
use reqwest::StatusCode;
use std::path::Path;

fn capability(install_state: ManagedBinaryInstallState) -> ManagedBinaryCapability {
    ManagedBinaryCapability {
        id: ManagedBinaryId::LlamaCpp,
        display_name: "llama.cpp".to_string(),
        install_state,
        available: matches!(
            install_state,
            ManagedBinaryInstallState::Installed | ManagedBinaryInstallState::SystemProvided
        ),
        can_install: install_state == ManagedBinaryInstallState::Missing,
        can_remove: install_state != ManagedBinaryInstallState::Unsupported,
        missing_files: Vec::new(),
        unavailable_reason: None,
    }
}

fn install_fake_runtime_files(dir: &Path, id: ManagedBinaryId) {
    std::fs::create_dir_all(dir).expect("create runtime dir");
    for file_name in definition(id).validate_installation(dir) {
        std::fs::write(dir.join(&file_name), [])
            .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
    }
    assert!(
        definition(id).validate_installation(dir).is_empty(),
        "fake runtime files should satisfy install validation"
    );
}

#[test]
fn download_response_mode_supports_partial_content_resume() {
    let mode = download_response_mode(
        64,
        StatusCode::PARTIAL_CONTENT,
        Some(64),
        Some("bytes 64-127/128"),
    )
    .expect("resume mode");

    assert_eq!(mode, DownloadResponseMode::Resume { total_size: 128 });
}

#[test]
fn download_response_mode_restarts_when_server_ignores_range() {
    let mode = download_response_mode(64, StatusCode::OK, Some(128), None).expect("fresh mode");

    assert_eq!(mode, DownloadResponseMode::Fresh { total_size: 128 });
}

#[test]
fn existing_download_artifact_uses_current_file_length() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
    std::fs::write(&artifact_path, vec![1_u8; 16]).expect("write retained artifact");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job_artifact = Some(ManagedRuntimePersistedJobArtifact {
        version: "b8248".to_string(),
        archive_name: "llama-b8248.tar.gz".to_string(),
        archive_path: artifact_path.display().to_string(),
        downloaded_bytes: 8,
        total_bytes: 32,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let artifact = existing_download_artifact(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        "llama-b8248.tar.gz",
    )
    .expect("load retained artifact")
    .expect("retained artifact");

    assert_eq!(artifact.temp_path, artifact_path);
    assert_eq!(artifact.downloaded_bytes, 16);
    assert_eq!(artifact.total_bytes, 32);
}

#[test]
fn readiness_state_maps_installed_runtime_to_ready() {
    let readiness =
        readiness_state_for_capability(&capability(ManagedBinaryInstallState::Installed));
    assert_eq!(readiness, ManagedRuntimeReadinessState::Ready);
}

#[test]
fn readiness_state_maps_missing_runtime_to_missing() {
    let readiness = readiness_state_for_capability(&capability(ManagedBinaryInstallState::Missing));
    assert_eq!(readiness, ManagedRuntimeReadinessState::Missing);
}

#[test]
fn snapshot_carries_additive_versions_and_selection_contracts() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let snapshot = snapshot_from_capability(
        temp_dir.path(),
        &capability(ManagedBinaryInstallState::Installed),
        None,
    );

    assert_eq!(snapshot.versions.len(), 1);
    assert_eq!(snapshot.selection.selected_version, None);
    assert_eq!(
        snapshot.readiness_state,
        ManagedRuntimeReadinessState::Ready
    );
    assert_eq!(
        snapshot.versions[0].runtime_key,
        ManagedBinaryId::LlamaCpp.key()
    );
    assert!(!snapshot.versions[0].platform_key.is_empty());
    assert!(!snapshot.versions[0].executable_name.is_empty());
    assert!(snapshot.active_job.is_none());
}

#[test]
fn persist_install_success_records_ready_version_and_selection() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");

    let state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");

    assert_eq!(runtime.versions.len(), 1);
    assert_eq!(runtime.versions[0].version, "b8248");
    assert_eq!(
        runtime.versions[0].runtime_key.as_deref(),
        Some(ManagedBinaryId::LlamaCpp.key())
    );
    assert!(runtime.versions[0].platform_key.is_some());
    assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
    assert_eq!(runtime.selection.active_version.as_deref(), Some("b8248"));
}

#[test]
fn persist_remove_success_clears_versions_and_selection() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");
    persist_remove_success(temp_dir.path(), ManagedBinaryId::LlamaCpp)
        .expect("persist remove success");

    let state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");

    assert!(runtime.versions.is_empty());
    assert_eq!(runtime.selection.selected_version, None);
    assert_eq!(runtime.selection.active_version, None);
}

#[test]
fn select_managed_runtime_version_updates_persisted_selection() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");
    select_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("b8248"))
        .expect("select runtime version");
    set_default_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("b8248"))
        .expect("set default runtime version");

    let state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");

    assert_eq!(runtime.selection.selected_version.as_deref(), Some("b8248"));
    assert_eq!(runtime.selection.default_version.as_deref(), Some("b8248"));
}

#[test]
fn select_managed_runtime_version_rejects_unknown_version() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");
    let error =
        select_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("other"))
            .expect_err("unknown version should fail");

    assert!(error.contains("is not installed"));
}

#[test]
fn select_managed_runtime_version_rejects_non_ready_version() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.versions.push(ManagedRuntimePersistedVersion {
        version: "b8248".to_string(),
        runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
        platform_key: Some("linux-x86_64".to_string()),
        readiness_state: ManagedRuntimeReadinessState::Failed,
        install_root: None,
        last_ready_at_ms: None,
        last_error: Some("validation failed".to_string()),
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let error =
        select_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("b8248"))
            .expect_err("non-ready version should fail");

    assert!(error.contains("is not ready for selection"));
}

#[test]
fn resolve_runtime_install_dir_uses_selected_version_install_root() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");
    select_managed_runtime_version(temp_dir.path(), ManagedBinaryId::LlamaCpp, Some("b8248"))
        .expect("select runtime version");

    let resolved_install_dir =
        resolve_runtime_install_dir(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("resolve install dir");

    assert_eq!(resolved_install_dir, install_dir);
}

#[test]
fn resolve_runtime_install_dir_rejects_missing_selected_version() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = state
        .runtimes
        .iter_mut()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");
    runtime.selection.selected_version = Some("other".to_string());
    runtime.versions = vec![ManagedRuntimePersistedVersion {
        version: "b8248".to_string(),
        runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
        platform_key: Some("linux-x86_64".to_string()),
        readiness_state: ManagedRuntimeReadinessState::Ready,
        install_root: Some(install_dir.display().to_string()),
        last_ready_at_ms: None,
        last_error: None,
    }];
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let error = resolve_runtime_install_dir(temp_dir.path(), ManagedBinaryId::LlamaCpp)
        .expect_err("missing selected version should fail");

    assert!(error.contains("selected version 'other' is not installed"));
}

#[test]
fn runtime_install_dir_for_projection_falls_back_when_selected_version_is_missing() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = state
        .runtimes
        .iter_mut()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");
    runtime.selection.selected_version = Some("other".to_string());
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let resolved_install_dir =
        runtime_install_dir_for_projection(temp_dir.path(), ManagedBinaryId::LlamaCpp)
            .expect("resolve projection install dir");

    assert_eq!(
        resolved_install_dir,
        temp_dir.path().join("runtimes/llama-cpp")
    );
}

#[test]
fn binary_capability_tolerates_stale_selected_version_state() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");

    persist_install_success(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        &install_dir,
        ManagedBinaryId::LlamaCpp.key(),
        definition(ManagedBinaryId::LlamaCpp).platform_key(),
    )
    .expect("persist install success");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = state
        .runtimes
        .iter_mut()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");
    runtime.selection.selected_version = Some("other".to_string());
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let capability =
        binary_capability(temp_dir.path(), ManagedBinaryId::LlamaCpp).expect("read capability");

    assert_eq!(capability.install_state, ManagedBinaryInstallState::Missing);
}

#[test]
fn managed_runtime_snapshot_uses_selected_version_failed_readiness() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp-b8248");
    install_fake_runtime_files(&install_dir, ManagedBinaryId::LlamaCpp);

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.selection.selected_version = Some("b8248".to_string());
    runtime.selection.default_version = Some("b8248".to_string());
    runtime.versions.push(ManagedRuntimePersistedVersion {
        version: "b8248".to_string(),
        runtime_key: Some(ManagedBinaryId::LlamaCpp.key().to_string()),
        platform_key: Some("linux-x86_64".to_string()),
        readiness_state: ManagedRuntimeReadinessState::Failed,
        install_root: Some(install_dir.display().to_string()),
        last_ready_at_ms: None,
        last_error: Some("validation failed".to_string()),
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let snapshot = crate::managed_runtime::managed_runtime_snapshot(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
    )
    .expect("managed runtime snapshot");

    assert_eq!(
        snapshot.readiness_state,
        ManagedRuntimeReadinessState::Failed
    );
    assert!(!snapshot.available);
    assert_eq!(
        snapshot.unavailable_reason.as_deref(),
        Some("validation failed")
    );
}

#[test]
fn managed_runtime_snapshot_uses_reconciled_interrupted_job_readiness() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let install_dir = temp_dir.path().join("runtimes/llama-cpp");
    install_fake_runtime_files(&install_dir, ManagedBinaryId::LlamaCpp);

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Downloading,
        status: "Downloading b8248".to_string(),
        current: 5,
        total: 10,
        resumable: true,
        cancellable: true,
        error: None,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let snapshot = crate::managed_runtime::managed_runtime_snapshot(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
    )
    .expect("managed runtime snapshot");

    assert_eq!(
        snapshot.readiness_state,
        ManagedRuntimeReadinessState::Failed
    );
    assert!(!snapshot.available);
    let active_job = snapshot.active_job.expect("reconciled active job");
    assert_eq!(active_job.state, ManagedRuntimeJobState::Failed);
    assert_eq!(active_job.status, "Interrupted before completion");
}

#[test]
fn cancel_binary_download_rejects_non_cancellable_jobs() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Extracting,
        status: "Extracting".to_string(),
        current: 10,
        total: 10,
        resumable: false,
        cancellable: false,
        error: None,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let error = cancel_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
        .expect_err("non-cancellable job should fail");

    assert!(error.contains("does not have a cancellable managed runtime job"));
}

#[test]
fn pause_binary_download_rejects_non_cancellable_jobs() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Extracting,
        status: "Extracting".to_string(),
        current: 10,
        total: 10,
        resumable: false,
        cancellable: false,
        error: None,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let error = pause_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
        .expect_err("non-cancellable job should fail");

    assert!(error.contains("does not have a pausable managed runtime job"));
}

#[test]
fn finish_requested_cancellation_persists_cancelled_job_and_history() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Downloading,
        status: "Downloading".to_string(),
        current: 32,
        total: 64,
        resumable: false,
        cancellable: true,
        error: None,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    cancel_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
        .expect("request cancellation");
    let cancelled = finish_requested_cancellation(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        32,
        64,
        None,
    )
    .expect("finish cancellation");

    assert!(cancelled);
    let state = load_managed_runtime_state(temp_dir.path()).expect("reload runtime state");
    let runtime = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");
    let active_job = runtime.active_job.as_ref().expect("cancelled job");
    assert_eq!(active_job.state, ManagedRuntimeJobState::Cancelled);
    assert_eq!(active_job.status, "Cancelled");
    assert_eq!(active_job.current, 32);
    assert_eq!(active_job.total, 64);
    assert!(runtime.active_job_artifact.is_none());
    assert_eq!(
        runtime
            .install_history
            .last()
            .map(|entry| entry.event.clone()),
        Some(ManagedRuntimeHistoryEventKind::Cancelled)
    );
}

#[test]
fn finish_requested_pause_persists_paused_job_and_history() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
    std::fs::write(&artifact_path, vec![1_u8; 32]).expect("write retained artifact");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Downloading,
        status: "Downloading".to_string(),
        current: 32,
        total: 64,
        resumable: true,
        cancellable: true,
        error: None,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    pause_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp).expect("request pause");
    let paused = finish_requested_pause(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
        "b8248",
        32,
        64,
        ManagedRuntimePersistedJobArtifact {
            version: "b8248".to_string(),
            archive_name: "llama-b8248.tar.gz".to_string(),
            archive_path: artifact_path.display().to_string(),
            downloaded_bytes: 32,
            total_bytes: 64,
        },
    )
    .expect("finish pause");

    assert!(paused);
    let state = load_managed_runtime_state(temp_dir.path()).expect("reload runtime state");
    let runtime = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");
    let active_job = runtime.active_job.as_ref().expect("paused job");
    assert_eq!(active_job.state, ManagedRuntimeJobState::Paused);
    assert_eq!(active_job.status, "Paused");
    assert_eq!(active_job.current, 32);
    assert_eq!(active_job.total, 64);
    assert!(active_job.resumable);
    assert!(runtime.active_job_artifact.is_some());
    assert_eq!(
        runtime
            .install_history
            .last()
            .map(|entry| entry.event.clone()),
        Some(ManagedRuntimeHistoryEventKind::Paused)
    );
}

#[test]
fn cancel_binary_download_discards_paused_artifact() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
    std::fs::write(&artifact_path, vec![1_u8; 32]).expect("write retained artifact");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Paused,
        status: "Paused".to_string(),
        current: 32,
        total: 64,
        resumable: true,
        cancellable: false,
        error: None,
    });
    runtime.active_job_artifact = Some(ManagedRuntimePersistedJobArtifact {
        version: "b8248".to_string(),
        archive_name: "llama-b8248.tar.gz".to_string(),
        archive_path: artifact_path.display().to_string(),
        downloaded_bytes: 32,
        total_bytes: 64,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    cancel_binary_download(temp_dir.path(), ManagedBinaryId::LlamaCpp)
        .expect("discard paused artifact");

    let state = load_managed_runtime_state(temp_dir.path()).expect("reload runtime state");
    let runtime = state
        .runtimes
        .iter()
        .find(|runtime| runtime.id == ManagedBinaryId::LlamaCpp)
        .expect("llama runtime state");
    let active_job = runtime.active_job.as_ref().expect("cancelled job");
    assert_eq!(active_job.state, ManagedRuntimeJobState::Cancelled);
    assert!(runtime.active_job_artifact.is_none());
    assert!(!artifact_path.exists());
    assert_eq!(
        runtime
            .install_history
            .last()
            .map(|entry| entry.event.clone()),
        Some(ManagedRuntimeHistoryEventKind::Cancelled)
    );
}

#[test]
fn managed_runtime_snapshot_projects_retained_job_artifact() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let artifact_path = temp_dir.path().join("partial-llama.tar.gz");
    std::fs::write(&artifact_path, []).expect("write retained artifact");

    let mut state = load_managed_runtime_state(temp_dir.path()).expect("load runtime state");
    let runtime = ensure_runtime_state_entry(&mut state, ManagedBinaryId::LlamaCpp);
    runtime.active_job = Some(ManagedRuntimeJobStatus {
        state: ManagedRuntimeJobState::Cancelled,
        status: "Cancelled".to_string(),
        current: 64,
        total: 128,
        resumable: true,
        cancellable: false,
        error: None,
    });
    runtime.active_job_artifact = Some(ManagedRuntimePersistedJobArtifact {
        version: "b8248".to_string(),
        archive_name: "llama-b8248.tar.gz".to_string(),
        archive_path: artifact_path.display().to_string(),
        downloaded_bytes: 64,
        total_bytes: 128,
    });
    save_managed_runtime_state(temp_dir.path(), &state).expect("save runtime state");

    let snapshot = crate::managed_runtime::managed_runtime_snapshot(
        temp_dir.path(),
        ManagedBinaryId::LlamaCpp,
    )
    .expect("managed runtime snapshot");

    let artifact = snapshot.job_artifact.expect("job artifact");
    assert_eq!(artifact.version, "b8248");
    assert_eq!(artifact.archive_name, "llama-b8248.tar.gz");
    assert_eq!(artifact.downloaded_bytes, 64);
    assert_eq!(artifact.total_bytes, 128);
    assert!(artifact.retained);
}
