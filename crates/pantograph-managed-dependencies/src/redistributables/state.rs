use std::fs;
use std::path::Path;

use super::contracts::{
    ManagedRedistributableId, ManagedRedistributablePersistedDependency,
    ManagedRedistributablePersistedState, ManagedRedistributableSelection,
};
use super::paths::{managed_redistributables_dir, redistributables_state_path, temp_state_path};

pub fn load_managed_redistributable_state(
    app_data_dir: &Path,
) -> Result<ManagedRedistributablePersistedState, String> {
    let path = redistributables_state_path(app_data_dir);
    if !path.exists() {
        return Ok(ManagedRedistributablePersistedState::default());
    }

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
