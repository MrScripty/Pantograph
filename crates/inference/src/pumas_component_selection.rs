//! Pumas-issued selected-byte custody, with an always-closed execution gate.
//!
//! This does not authenticate provider/build declarations or observe Python.
//! Blocking source validation owns its selection on this stack and runs before
//! acquiring the startup broker mutex; no detached effect is created here.
use crate::managed_python_binding::ManagedPythonRuntimeStartRequest;
use crate::python_startup_broker::{
    PythonStartupBroker, PythonStartupReservation, PythonStartupReservationRefusal,
};
use pumas_app_manager::version_manager::TorchComponentSelection;
use std::{fmt, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumasComponentStartRefusal {
    InvalidDeclaration,
    SelectionMismatch,
    InvalidRetainedBytes,
    Reservation(PythonStartupReservationRefusal),
    AssembledUnqualified,
    StartupEvidenceUnavailable,
}

impl fmt::Display for PumasComponentStartRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeclaration => f.write_str("managed component declaration is invalid"),
            Self::SelectionMismatch => f.write_str("Pumas selected revision/manifest/depot mismatch"),
            Self::InvalidRetainedBytes => f.write_str("Pumas retained byte association is invalid"),
            Self::Reservation(error) => error.fmt(f),
            Self::AssembledUnqualified => f.write_str(
                "Pumas component is assembled_unqualified: no runnable interpreter or initialized import provenance",
            ),
            Self::StartupEvidenceUnavailable => f.write_str(
                "managed component has no coordinated interpreter/provider startup evidence",
            ),
        }
    }
}
impl std::error::Error for PumasComponentStartRefusal {}

/// Selected bytes plus an exclusive pre-exposure broker generation. The owner
/// selection stays genuine and cloneable; this transaction is not cloneable.
/// The environment/configuration and provider identities are still declarations.
pub struct PumasComponentReservation {
    selection: TorchComponentSelection,
    reservation: PythonStartupReservation,
}

impl PumasComponentReservation {
    pub fn selection(&self) -> &TorchComponentSelection {
        &self.selection
    }

    /// No disposition, including any future qualified value, authorizes startup
    /// without the separate coordinated initialization/import evidence contract.
    pub fn closed_start_refusal(&self) -> PumasComponentStartRefusal {
        if self.selection.qualification() == "assembled_unqualified" {
            PumasComponentStartRefusal::AssembledUnqualified
        } else {
            PumasComponentStartRefusal::StartupEvidenceUnavailable
        }
    }

    /// Retain the actual source Arc in process custody. This returns no native
    /// execution permission; the production closed preflight never calls it.
    pub fn pin_for_process(self) -> Result<(), PumasComponentStartRefusal> {
        self.reservation
            .pin_for_process()
            .map_err(PumasComponentStartRefusal::Reservation)
    }
}

impl PythonStartupBroker {
    /// Synchronous, blocking byte preflight. Invoke on an appropriate caller-owned
    /// thread; an async host must keep the selection in its registered blocking
    /// effect and drain that effect. This function neither spawns nor opens Python.
    /// Legacy startup can win during validation, so admission is checked afterwards.
    pub fn reserve_component_selection(
        self: &Arc<Self>,
        declaration: &ManagedPythonRuntimeStartRequest,
        expected_interpreter_depot_manifest_sha256: &str,
        selection: TorchComponentSelection,
    ) -> Result<PumasComponentReservation, PumasComponentStartRefusal> {
        reserve_with_validation(
            self,
            declaration,
            expected_interpreter_depot_manifest_sha256,
            selection,
            |selection| selection.validate().map_err(|_| ()),
        )
    }
}

// Private factoring lets controlled tests place a barrier around real validation
// to observe a legacy winner. Production always calls the owner's actual validate.
fn reserve_with_validation(
    broker: &Arc<PythonStartupBroker>,
    declaration: &ManagedPythonRuntimeStartRequest,
    expected_depot: &str,
    selection: TorchComponentSelection,
    validate: impl FnOnce(&TorchComponentSelection) -> Result<(), ()>,
) -> Result<PumasComponentReservation, PumasComponentStartRefusal> {
    if declaration.validate().is_err()
        || expected_depot.len() != 64
        || !expected_depot
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PumasComponentStartRefusal::InvalidDeclaration);
    }
    if declaration.runtime_revision != selection.revision_tag()
        || declaration.runtime_manifest_sha256 != selection.manifest_sha256()
        || expected_depot != selection.interpreter_depot_manifest_sha256()
    {
        return Err(PumasComponentStartRefusal::SelectionMismatch);
    }
    validate(&selection).map_err(|_| PumasComponentStartRefusal::InvalidRetainedBytes)?;
    let reservation = broker
        .reserve_declared(declaration, selection.retained_source().clone())
        .map_err(PumasComponentStartRefusal::Reservation)?;
    Ok(PumasComponentReservation {
        selection,
        reservation,
    })
}

#[cfg(test)]
#[path = "pumas_component_selection_tests.rs"]
mod tests;
