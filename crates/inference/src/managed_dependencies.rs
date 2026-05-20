use std::path::Path;

use pantograph_managed_dependencies::{
    ManagedDependencyCategory, ManagedDependencyKey, ManagedDependencyStatus,
    MediaToolDependencyId, NativeArtifactDependencyId, ResolvedManagedDependencyCommand,
    RuntimeSidecarDependencyId,
};

use crate::managed_redistributables::list_managed_dependency_statuses;
use crate::managed_runtime::{
    list_managed_runtime_dependency_statuses, resolve_runtime_sidecar_dependency_command,
};
use crate::ManagedBinaryId;

pub fn list_all_managed_dependency_statuses(
    app_data_dir: &Path,
) -> Result<Vec<ManagedDependencyStatus>, String> {
    let mut statuses = list_managed_runtime_dependency_statuses(app_data_dir)?;
    statuses.extend(list_managed_dependency_statuses(app_data_dir));
    statuses.sort_by(|left, right| {
        managed_dependency_sort_key(left).cmp(&managed_dependency_sort_key(right))
    });
    Ok(statuses)
}

pub fn resolve_managed_dependency_command(
    app_data_dir: &Path,
    key: ManagedDependencyKey,
    args: &[&str],
) -> Result<ResolvedManagedDependencyCommand, String> {
    match key {
        ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp) => {
            resolve_runtime_sidecar_dependency_command(
                app_data_dir,
                ManagedBinaryId::LlamaCpp,
                args,
            )
        }
        ManagedDependencyKey::MediaTool(id) => Err(format!(
            "{} media tool command resolution is owned by the media conversion boundary",
            media_tool_display_name(id)
        )),
        ManagedDependencyKey::NativeArtifact(id) => Err(format!(
            "{} native artifact activation is not executable command resolution",
            native_artifact_display_name(id)
        )),
    }
}

fn managed_dependency_sort_key(status: &ManagedDependencyStatus) -> (u8, &'static str) {
    let category_order = match status.category {
        ManagedDependencyCategory::RuntimeSidecar => 0,
        ManagedDependencyCategory::MediaTool => 1,
        ManagedDependencyCategory::NativeArtifact => 2,
    };
    (category_order, status.key.stable_key())
}

fn media_tool_display_name(id: MediaToolDependencyId) -> &'static str {
    match id {
        MediaToolDependencyId::Ffmpeg => "FFmpeg",
        MediaToolDependencyId::Ocioconvert => "ocioconvert",
        MediaToolDependencyId::Oiiotool => "oiiotool",
    }
}

fn native_artifact_display_name(id: NativeArtifactDependencyId) -> &'static str {
    match id {
        NativeArtifactDependencyId::OpenColorIo => "OpenColorIO",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_all_managed_dependency_statuses_combines_runtime_and_media_facts() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        let statuses =
            list_all_managed_dependency_statuses(temp_dir.path()).expect("managed dependency list");

        assert_eq!(statuses.len(), 5);
        assert_eq!(
            statuses[0].key,
            ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp)
        );
        assert_eq!(
            statuses[1].key,
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ffmpeg)
        );
        assert_eq!(
            statuses[4].key,
            ManagedDependencyKey::NativeArtifact(NativeArtifactDependencyId::OpenColorIo)
        );
    }

    #[test]
    fn resolve_managed_dependency_command_keeps_media_tools_out_of_inference() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        let media_error = resolve_managed_dependency_command(
            temp_dir.path(),
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ffmpeg),
            &["-version"],
        )
        .expect_err("media tool command resolution should stay outside inference");

        assert!(media_error.contains("media conversion boundary"));

        let native_error = resolve_managed_dependency_command(
            temp_dir.path(),
            ManagedDependencyKey::NativeArtifact(NativeArtifactDependencyId::OpenColorIo),
            &[],
        )
        .expect_err("native artifacts should not resolve as commands");

        assert!(native_error.contains("not executable command resolution"));
    }

    #[test]
    fn resolve_managed_dependency_command_routes_runtime_sidecars() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        let error = resolve_managed_dependency_command(
            temp_dir.path(),
            ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp),
            &["--port", "8080"],
        )
        .expect_err("empty temp root has no selected llama.cpp version");

        assert!(
            error.contains("does not have managed runtime state"),
            "unexpected runtime sidecar command error: {error}"
        );
    }
}
