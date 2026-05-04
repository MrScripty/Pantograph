use std::fs;
use std::path::Path;

use super::contracts::{
    ManagedRedistributableId, ManagedRedistributablePersistedDependency,
    ManagedRedistributablePersistedState, ManagedRedistributableSelection,
};
use super::paths::{
    legacy_redistributables_state_path, managed_redistributables_dir, redistributables_state_path,
    temp_state_path,
};

pub fn load_managed_redistributable_state(
    app_data_dir: &Path,
) -> Result<ManagedRedistributablePersistedState, String> {
    let path = redistributables_state_path(app_data_dir);
    if path.exists() {
        return load_state_file(&path);
    }

    let legacy_path = legacy_redistributables_state_path(app_data_dir);
    if legacy_path.exists() {
        let mut state = load_state_file(&legacy_path)?;
        clear_imported_legacy_leases(&mut state);
        return Ok(state);
    }

    Ok(ManagedRedistributablePersistedState::default())
}

fn load_state_file(path: &Path) -> Result<ManagedRedistributablePersistedState, String> {
    let contents = fs::read_to_string(&path).map_err(|e| {
        format!(
            "Failed to read managed redistributable state {:?}: {}",
            path, e
        )
    })?;
    let mut state: ManagedRedistributablePersistedState =
        serde_json::from_str(&contents).map_err(|e| {
            format!(
                "Failed to parse managed redistributable state {:?}: {}",
                path, e
            )
        })?;
    if state.schema_version == 0 {
        state.schema_version = ManagedRedistributablePersistedState::default().schema_version;
    }

    Ok(state)
}

fn clear_imported_legacy_leases(state: &mut ManagedRedistributablePersistedState) {
    for dependency in &mut state.dependencies {
        dependency.active_leases.clear();
    }
}

pub fn save_managed_redistributable_state(
    app_data_dir: &Path,
    state: &ManagedRedistributablePersistedState,
) -> Result<(), String> {
    let root = managed_redistributables_dir(app_data_dir);
    fs::create_dir_all(&root).map_err(|e| {
        format!(
            "Failed to create managed redistributable directory {:?}: {}",
            root, e
        )
    })?;

    let path = redistributables_state_path(app_data_dir);
    let temp_path = temp_state_path(&path);
    let contents = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize managed redistributable state: {}", e))?;
    fs::write(&temp_path, contents).map_err(|e| {
        format!(
            "Failed to write managed redistributable temp state {:?}: {}",
            temp_path, e
        )
    })?;
    fs::rename(&temp_path, &path).map_err(|e| {
        format!(
            "Failed to finalize managed redistributable state {:?}: {}",
            path, e
        )
    })
}

pub(crate) fn ensure_persisted_dependency(
    state: &mut ManagedRedistributablePersistedState,
    id: ManagedRedistributableId,
) -> &mut ManagedRedistributablePersistedDependency {
    if let Some(index) = state
        .dependencies
        .iter()
        .position(|dependency| dependency.id == id)
    {
        return &mut state.dependencies[index];
    }

    state
        .dependencies
        .push(ManagedRedistributablePersistedDependency {
            id,
            selection: ManagedRedistributableSelection::default(),
            active_leases: Vec::new(),
        });
    state
        .dependencies
        .last_mut()
        .expect("managed redistributable state entry should exist after push")
}

pub(crate) fn persisted_dependency(
    state: &ManagedRedistributablePersistedState,
    id: ManagedRedistributableId,
) -> Option<&ManagedRedistributablePersistedDependency> {
    state
        .dependencies
        .iter()
        .find(|dependency| dependency.id == id)
}

pub(crate) fn persisted_dependency_mut(
    state: &mut ManagedRedistributablePersistedState,
    id: ManagedRedistributableId,
) -> Option<&mut ManagedRedistributablePersistedDependency> {
    state
        .dependencies
        .iter_mut()
        .find(|dependency| dependency.id == id)
}

pub(crate) fn clear_selection_version(selection: &mut Option<String>, version: &str) {
    if selection
        .as_deref()
        .is_some_and(|selected| selected == version)
    {
        *selection = None;
    }
}

#[cfg(test)]
mod tests {
    use super::super::paths::{legacy_redistributables_state_path, redistributables_state_path};
    use super::*;
    use serde_json::json;

    #[test]
    fn load_managed_redistributable_state_imports_legacy_state_when_canonical_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let legacy_path = legacy_redistributables_state_path(temp_dir.path());
        std::fs::create_dir_all(legacy_path.parent().expect("legacy state parent"))
            .expect("create legacy state dir");
        std::fs::write(
            &legacy_path,
            json!({
                "schema_version": 0,
                "dependencies": [{
                    "id": "ffmpeg",
                    "selection": {
                        "selected_version": "n7.1.1",
                        "active_version": "n7.1.1",
                        "default_version": "n7.1.1"
                    }
                }]
            })
            .to_string(),
        )
        .expect("write legacy state");

        let state =
            load_managed_redistributable_state(temp_dir.path()).expect("load imported state");

        assert_eq!(
            state.schema_version,
            ManagedRedistributablePersistedState::default().schema_version
        );
        assert_eq!(state.dependencies.len(), 1);
        assert_eq!(state.dependencies[0].id, ManagedRedistributableId::Ffmpeg);
        assert_eq!(
            state.dependencies[0].selection.selected_version.as_deref(),
            Some("n7.1.1")
        );
    }

    #[test]
    fn load_managed_redistributable_state_prefers_canonical_state_over_legacy() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let canonical_path = redistributables_state_path(temp_dir.path());
        let legacy_path = legacy_redistributables_state_path(temp_dir.path());
        std::fs::create_dir_all(canonical_path.parent().expect("canonical state parent"))
            .expect("create canonical state dir");
        std::fs::create_dir_all(legacy_path.parent().expect("legacy state parent"))
            .expect("create legacy state dir");
        std::fs::write(
            &canonical_path,
            json!({
                "schema_version": 1,
                "dependencies": [{
                    "id": "ocioconvert",
                    "selection": {
                        "selected_version": "2.4.2",
                        "active_version": "2.4.2",
                        "default_version": "2.4.2"
                    }
                }]
            })
            .to_string(),
        )
        .expect("write canonical state");
        std::fs::write(
            &legacy_path,
            json!({
                "schema_version": 1,
                "dependencies": [{
                    "id": "ffmpeg",
                    "selection": {
                        "selected_version": "n7.1.1",
                        "active_version": "n7.1.1",
                        "default_version": "n7.1.1"
                    }
                }]
            })
            .to_string(),
        )
        .expect("write legacy state");

        let state =
            load_managed_redistributable_state(temp_dir.path()).expect("load canonical state");

        assert_eq!(state.dependencies.len(), 1);
        assert_eq!(
            state.dependencies[0].id,
            ManagedRedistributableId::Ocioconvert
        );
        assert_eq!(
            state.dependencies[0].selection.selected_version.as_deref(),
            Some("2.4.2")
        );
    }

    #[test]
    fn load_managed_redistributable_state_drops_legacy_active_leases() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let legacy_path = legacy_redistributables_state_path(temp_dir.path());
        std::fs::create_dir_all(legacy_path.parent().expect("legacy state parent"))
            .expect("create legacy state dir");
        std::fs::write(
            &legacy_path,
            json!({
                "schema_version": 1,
                "dependencies": [{
                    "id": "ffmpeg",
                    "selection": {
                        "selected_version": "n7.1.1",
                        "active_version": "n7.1.1",
                        "default_version": "n7.1.1"
                    },
                    "active_leases": [{
                        "id": "legacy-lease",
                        "version": "n7.1.1",
                        "holder": "legacy-process",
                        "acquired_at_ms": 1
                    }]
                }]
            })
            .to_string(),
        )
        .expect("write legacy state");

        let state =
            load_managed_redistributable_state(temp_dir.path()).expect("load imported state");

        assert!(state.dependencies[0].active_leases.is_empty());
        assert_eq!(
            state.dependencies[0].selection.active_version.as_deref(),
            Some("n7.1.1")
        );
    }
}
