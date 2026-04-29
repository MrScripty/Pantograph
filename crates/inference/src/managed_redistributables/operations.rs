use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::catalog::{
    catalog_entry, catalog_supported, managed_redistributable_catalog, missing_expected_files,
    validate_catalog_version, validate_expected_files,
};
use super::contracts::{
    ManagedRedistributableCatalogEntry, ManagedRedistributableId,
    ManagedRedistributableInstallState, ManagedRedistributableLease,
    ManagedRedistributableLeaseToken, ManagedRedistributableReadiness,
    ManagedRedistributableStatus, ManagedRedistributableVersionStatus,
};
use super::paths::{
    current_unix_timestamp_ms, managed_redistributable_version_dir, sanitize_path_segment,
};
use super::state::{
    clear_selection_version, ensure_persisted_dependency, load_managed_redistributable_state,
    persisted_dependency, persisted_dependency_mut, save_managed_redistributable_state,
};

pub fn managed_redistributable_status(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
) -> ManagedRedistributableStatus {
    let catalog = catalog_entry(id);
    let state = load_managed_redistributable_state(app_data_dir).unwrap_or_default();
    status_from_catalog(app_data_dir, catalog, &state)
}

pub fn list_managed_redistributable_statuses(
    app_data_dir: &Path,
) -> Vec<ManagedRedistributableStatus> {
    let state = load_managed_redistributable_state(app_data_dir).unwrap_or_default();
    managed_redistributable_catalog()
        .into_iter()
        .map(|catalog| status_from_catalog(app_data_dir, catalog, &state))
        .collect()
}

pub fn install_managed_redistributable_from_staging(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
    staging_dir: &Path,
) -> Result<PathBuf, String> {
    let catalog = validate_catalog_version(id, version)?;
    validate_expected_files(staging_dir, &catalog)?;

    let install_root = managed_redistributable_version_dir(app_data_dir, id, version);
    let install_parent = install_root.parent().ok_or_else(|| {
        format!(
            "Managed redistributable install root {:?} has no parent",
            install_root
        )
    })?;
    fs::create_dir_all(install_parent).map_err(|e| {
        format!(
            "Failed to create managed redistributable versions directory {:?}: {}",
            install_parent, e
        )
    })?;

    let staging_install = install_parent.join(format!(
        ".installing-{}-{}",
        sanitize_path_segment(version),
        uuid::Uuid::new_v4()
    ));
    copy_dir_all(staging_dir, &staging_install).map_err(|e| {
        format!(
            "Failed to stage managed redistributable from {:?} to {:?}: {}",
            staging_dir, staging_install, e
        )
    })?;
    if let Err(error) = validate_expected_files(&staging_install, &catalog) {
        let _ = fs::remove_dir_all(&staging_install);
        return Err(error);
    }

    if install_root.exists() {
        fs::remove_dir_all(&install_root).map_err(|e| {
            format!(
                "Failed to replace existing managed redistributable install {:?}: {}",
                install_root, e
            )
        })?;
    }
    fs::rename(&staging_install, &install_root).map_err(|e| {
        format!(
            "Failed to finalize managed redistributable install {:?}: {}",
            install_root, e
        )
    })?;

    Ok(install_root)
}

pub fn select_managed_redistributable_version(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: Option<&str>,
) -> Result<(), String> {
    update_managed_redistributable_selection(app_data_dir, id, version, SelectionTarget::Selected)
}

pub fn set_default_managed_redistributable_version(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: Option<&str>,
) -> Result<(), String> {
    update_managed_redistributable_selection(app_data_dir, id, version, SelectionTarget::Default)
}

pub fn activate_managed_redistributable_version(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
) -> Result<(), String> {
    let catalog = validate_catalog_version(id, version)?;
    let install_root = managed_redistributable_version_dir(app_data_dir, id, version);
    validate_expected_files(&install_root, &catalog)?;

    let mut state = load_managed_redistributable_state(app_data_dir)?;
    let dependency = ensure_persisted_dependency(&mut state, id);
    dependency.selection.active_version = Some(version.to_string());
    save_managed_redistributable_state(app_data_dir, &state)
}

pub fn acquire_managed_redistributable_lease(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    holder: &str,
) -> Result<ManagedRedistributableLeaseToken, String> {
    let mut state = load_managed_redistributable_state(app_data_dir)?;
    let dependency = ensure_persisted_dependency(&mut state, id);
    let version = dependency
        .selection
        .active_version
        .clone()
        .ok_or_else(|| format!("{} does not have an active version", id.display_name()))?;
    let catalog = validate_catalog_version(id, &version)?;
    let install_root = managed_redistributable_version_dir(app_data_dir, id, &version);
    validate_expected_files(&install_root, &catalog)?;

    let lease_id = uuid::Uuid::new_v4().to_string();
    dependency.active_leases.push(ManagedRedistributableLease {
        id: lease_id.clone(),
        version: version.clone(),
        holder: holder.to_string(),
        acquired_at_ms: current_unix_timestamp_ms(),
    });
    save_managed_redistributable_state(app_data_dir, &state)?;

    Ok(ManagedRedistributableLeaseToken {
        id,
        version,
        lease_id,
    })
}

pub fn release_managed_redistributable_lease(
    app_data_dir: &Path,
    token: &ManagedRedistributableLeaseToken,
) -> Result<(), String> {
    let mut state = load_managed_redistributable_state(app_data_dir)?;
    let Some(dependency) = persisted_dependency_mut(&mut state, token.id) else {
        return Err(format!(
            "{} does not have a managed redistributable state entry",
            token.id.display_name()
        ));
    };
    let original_len = dependency.active_leases.len();
    dependency
        .active_leases
        .retain(|lease| lease.id != token.lease_id || lease.version != token.version);
    if dependency.active_leases.len() == original_len {
        return Err(format!(
            "Managed redistributable lease {} for {} {} was not found",
            token.lease_id,
            token.id.display_name(),
            token.version
        ));
    }
    save_managed_redistributable_state(app_data_dir, &state)
}

pub fn remove_managed_redistributable_version(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
) -> Result<(), String> {
    validate_catalog_version(id, version)?;
    let mut state = load_managed_redistributable_state(app_data_dir)?;
    if let Some(dependency) = persisted_dependency(&state, id) {
        let active = dependency
            .selection
            .active_version
            .as_deref()
            .is_some_and(|active_version| active_version == version);
        let active_lease_count = dependency
            .active_leases
            .iter()
            .filter(|lease| lease.version == version)
            .count();
        if active && active_lease_count > 0 {
            return Err(format!(
                "Refusing to remove active {} {} while {} lease(s) exist",
                id.display_name(),
                version,
                active_lease_count
            ));
        }
    }

    let install_root = managed_redistributable_version_dir(app_data_dir, id, version);
    if install_root.exists() {
        fs::remove_dir_all(&install_root).map_err(|e| {
            format!(
                "Failed to remove managed redistributable install {:?}: {}",
                install_root, e
            )
        })?;
    }

    if let Some(dependency) = persisted_dependency_mut(&mut state, id) {
        dependency
            .active_leases
            .retain(|lease| lease.version != version);
        clear_selection_version(&mut dependency.selection.selected_version, version);
        clear_selection_version(&mut dependency.selection.default_version, version);
        clear_selection_version(&mut dependency.selection.active_version, version);
    }
    save_managed_redistributable_state(app_data_dir, &state)
}

fn status_from_catalog(
    app_data_dir: &Path,
    catalog: ManagedRedistributableCatalogEntry,
    state: &super::contracts::ManagedRedistributablePersistedState,
) -> ManagedRedistributableStatus {
    let install_root =
        managed_redistributable_version_dir(app_data_dir, catalog.id, &catalog.version);
    let missing_files = missing_expected_files(&install_root, &catalog.expected_files);
    let supported = catalog_supported(&catalog);
    let ready = supported && missing_files.is_empty();
    let install_state = if !supported {
        ManagedRedistributableInstallState::Unsupported
    } else if ready {
        ManagedRedistributableInstallState::Installed
    } else {
        ManagedRedistributableInstallState::Missing
    };
    let readiness = if !supported {
        ManagedRedistributableReadiness::Unsupported
    } else if ready {
        ManagedRedistributableReadiness::Ready
    } else {
        ManagedRedistributableReadiness::Missing
    };
    let selection = persisted_dependency(state, catalog.id)
        .map(|dependency| dependency.selection.clone())
        .unwrap_or_default();

    ManagedRedistributableStatus {
        id: catalog.id,
        display_name: catalog.display_name.clone(),
        category: catalog.category,
        install_state,
        readiness,
        available: ready,
        missing_files: missing_files.clone(),
        versions: vec![ManagedRedistributableVersionStatus {
            version: catalog.version.clone(),
            platform_key: catalog.platform_key.clone(),
            install_root: install_root.display().to_string(),
            expected_files: catalog.expected_files.clone(),
            missing_files,
            install_state,
            readiness,
            selected: selection
                .selected_version
                .as_deref()
                .is_some_and(|version| version == catalog.version),
            active: selection
                .active_version
                .as_deref()
                .is_some_and(|version| version == catalog.version),
        }],
        catalog,
        selection,
    }
}

fn update_managed_redistributable_selection(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: Option<&str>,
    target: SelectionTarget,
) -> Result<(), String> {
    if let Some(version) = version {
        let catalog = validate_catalog_version(id, version)?;
        let install_root = managed_redistributable_version_dir(app_data_dir, id, version);
        validate_expected_files(&install_root, &catalog)?;
    } else {
        validate_catalog_version(id, &catalog_entry(id).version)?;
    }

    let mut state = load_managed_redistributable_state(app_data_dir)?;
    let dependency = ensure_persisted_dependency(&mut state, id);
    let target_field = match target {
        SelectionTarget::Selected => &mut dependency.selection.selected_version,
        SelectionTarget::Default => &mut dependency.selection.default_version,
    };
    *target_field = version.map(ToString::to_string);
    save_managed_redistributable_state(app_data_dir, &state)
}

fn copy_dir_all(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination_path = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination_path)?;
        }
    }
    Ok(())
}

enum SelectionTarget {
    Selected,
    Default,
}
