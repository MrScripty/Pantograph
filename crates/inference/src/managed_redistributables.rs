mod catalog;
mod contracts;
mod neutral_contracts;
mod operations;
mod paths;
mod state;

pub use catalog::{managed_redistributable_catalog, managed_redistributable_catalog_entry};
pub use contracts::{
    ManagedRedistributableArchiveKind, ManagedRedistributableCatalogEntry,
    ManagedRedistributableCategory, ManagedRedistributableId, ManagedRedistributableInstallState,
    ManagedRedistributableLease, ManagedRedistributableLeaseToken,
    ManagedRedistributablePackageKind, ManagedRedistributablePersistedDependency,
    ManagedRedistributablePersistedState, ManagedRedistributableReadiness,
    ManagedRedistributableSelection, ManagedRedistributableSource, ManagedRedistributableStatus,
    ManagedRedistributableVersionStatus,
};
pub use neutral_contracts::{list_managed_dependency_statuses, managed_dependency_status};
pub use operations::{
    acquire_managed_redistributable_lease, activate_managed_redistributable_version,
    install_managed_redistributable_from_staging, list_managed_redistributable_statuses,
    managed_redistributable_status, release_managed_redistributable_lease,
    remove_managed_redistributable_version, select_managed_redistributable_version,
    set_default_managed_redistributable_version,
};
pub use paths::managed_redistributables_dir;
pub use state::{load_managed_redistributable_state, save_managed_redistributable_state};
