use std::path::Path;

use pantograph_managed_dependencies::{ManagedDependencyCategory, ManagedDependencyStatus};

use crate::managed_redistributables::list_managed_dependency_statuses;
use crate::managed_runtime::list_managed_runtime_dependency_statuses;

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

fn managed_dependency_sort_key(status: &ManagedDependencyStatus) -> (u8, &'static str) {
    let category_order = match status.category {
        ManagedDependencyCategory::RuntimeSidecar => 0,
        ManagedDependencyCategory::MediaTool => 1,
        ManagedDependencyCategory::NativeArtifact => 2,
    };
    (category_order, status.key.stable_key())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_managed_dependencies::{
        ManagedDependencyKey, MediaToolDependencyId, NativeArtifactDependencyId,
        RuntimeSidecarDependencyId,
    };

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
}
